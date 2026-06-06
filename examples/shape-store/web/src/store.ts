import productCatalogJson from "./catalog.json";

export const categoryTabs = ["NEW", "MENS", "WOMENS", "FOOTWEAR", "ACCESSORIES", "SLIDES"] as const;
export const productSizes = ["4", "5", "6", "7", "8", "9", "10", "11", "12", "13", "14", "15", "16"] as const;
export const shippingCents = 0;
export const freeShippingThresholdCents = 20000;

const apiBaseUrl = import.meta.env.VITE_API_URL ?? "/api";
const currencyFormatter = new Intl.NumberFormat("en-US", {
  currency: "USD",
  style: "currency",
});
const wholeDollarFormatter = new Intl.NumberFormat("en-US", {
  currency: "USD",
  maximumFractionDigits: 0,
  minimumFractionDigits: 0,
  style: "currency",
});

export type Category = (typeof categoryTabs)[number];
export type CartQuantities = Record<string, number>;
export type LoadState = "loading" | "ready" | "error";

export type Product = {
  categories: readonly Category[];
  galleryImages: readonly string[];
  id: string;
  image: string;
  inventory: number;
  name: string;
  priceCents: number;
};

export type CartLine = {
  product: Product;
  quantity: number;
  selectedSize?: string;
};

export type CheckoutFields = {
  apartment: string;
  address: string;
  city: string;
  country: string;
  email: string;
  firstName: string;
  lastName: string;
  phone: string;
};

export type OrderReceipt = {
  id: string;
  totalCents: number;
};

type StripeCheckoutResult =
  | {
      checkoutUrl: string;
      mode: "stripe";
    }
  | {
      message: string;
      mode: "demo";
      orderId: string;
    };

type RawProduct = Omit<Product, "galleryImages"> & {
  galleryImages?: readonly string[];
};

type ProductCatalog = {
  products: RawProduct[];
};

type ProductsResponse = {
  products: RawProduct[];
};

type CheckoutApiResponse = {
  checkoutUrl?: string;
  error?: string;
  message?: string;
  mode?: "demo" | "stripe";
  orderId?: string;
  totalCents?: number;
};

type CartApiResponse = {
  data: CheckoutApiResponse;
  ok: boolean;
};

const productCatalog = productCatalogJson as ProductCatalog;

const officialProductMediaBaseUrl = "https://media.tovuk.app/shape-store/products";
const configuredProductMediaBaseUrl = normalizeProductMediaBaseUrl(import.meta.env.VITE_PRODUCT_MEDIA_BASE_URL ?? "");
const productMediaBaseUrl =
  configuredProductMediaBaseUrl || (globalThis.location?.hostname === "shape-store.tovuk.app" ? officialProductMediaBaseUrl : "");
const productGalleryViews = ["front", "angle", "detail"] as const;

export const fallbackProducts: Product[] = shapeProducts(productCatalog.products);

export async function fetchProducts() {
  const response = await fetch(`${apiBaseUrl}/products`);
  if (!response.ok) {
    throw new Error(`Product API returned ${response.status}`);
  }

  const data = (await response.json()) as ProductsResponse;
  if (!hasProducts(data)) {
    throw new Error("Product API returned an empty catalog");
  }

  return shapeProducts(data.products);
}

export async function reserveOrder(checkoutFields: CheckoutFields, cartLines: CartLine[], totalCents: number) {
  const response = await postCartRequest("orders", checkoutFields, cartLines, totalCents);
  if (!response.ok) {
    throw new Error("Order API did not return a receipt");
  }
  return orderReceipt(response.data);
}

export async function createStripeCheckout(
  checkoutFields: CheckoutFields,
  cartLines: CartLine[],
  totalCents: number,
) {
  const response = await postCartRequest("checkout", checkoutFields, cartLines, totalCents);
  if (!response.ok) {
    throw new Error(response.data.error ?? "Stripe checkout failed");
  }
  return stripeCheckoutResult(response.data);
}

async function postCartRequest(
  path: "checkout" | "orders",
  checkoutFields: CheckoutFields,
  cartLines: CartLine[],
  totalCents: number,
): Promise<CartApiResponse> {
  const response = await fetch(`${apiBaseUrl}/${path}`, {
    body: JSON.stringify({
      customer: checkoutFields,
      items: orderItemsFromCart(cartLines),
      totalCents,
    }),
    headers: { "Content-Type": "application/json" },
    method: "POST",
  });
  return {
    data: (await response.json()) as CheckoutApiResponse,
    ok: response.ok,
  };
}

function orderItemsFromCart(cartLines: CartLine[]) {
  return cartLines.map((line) => ({
    productId: line.product.id,
    quantity: line.quantity,
  }));
}

function stripeCheckoutResult(data: CheckoutApiResponse): StripeCheckoutResult {
  const result = stripeRedirectResult(data) ?? stripeDemoResult(data);
  if (result === null) {
    throw new Error("Checkout API did not return a supported payment result");
  }
  return result;
}

function orderReceipt(data: CheckoutApiResponse): OrderReceipt {
  if (typeof data.orderId !== "string" || typeof data.totalCents !== "number") {
    throw new Error("Order API did not return a receipt");
  }
  return { id: data.orderId, totalCents: data.totalCents };
}

function stripeRedirectResult(data: CheckoutApiResponse): StripeCheckoutResult | null {
  return data.mode === "stripe" && data.checkoutUrl ? { checkoutUrl: data.checkoutUrl, mode: "stripe" } : null;
}

function stripeDemoResult(data: CheckoutApiResponse): StripeCheckoutResult | null {
  return data.mode === "demo" ? stripeDemoPayload(data) : null;
}

function stripeDemoPayload(data: CheckoutApiResponse): StripeCheckoutResult | null {
  if (typeof data.orderId !== "string" || typeof data.message !== "string") {
    return null;
  }
  return { message: data.message, mode: "demo", orderId: data.orderId };
}

export function formatCurrency(cents: number) {
  const formatter = cents % 100 === 0 ? wholeDollarFormatter : currencyFormatter;
  return formatter.format(cents / 100);
}

function shapeProducts(products: readonly RawProduct[]): Product[] {
  return products.map(shapeProduct);
}

function shapeProduct(product: RawProduct): Product {
  const image = productImageUrl(product.image);
  return {
    ...product,
    galleryImages: galleryImagesForProduct(image, product.galleryImages),
    image,
  };
}

function galleryImagesForProduct(image: string, galleryImages: readonly string[] | undefined) {
  const explicitImages = galleryImages?.filter(Boolean).map(productImageUrl);
  if (explicitImages !== undefined && explicitImages.length > 0) {
    return explicitImages;
  }

  return productGalleryViews.map((view) => galleryViewUrl(image, view));
}

function galleryViewUrl(image: string, view: (typeof productGalleryViews)[number]) {
  return view === "front" ? image : `${image}${image.includes("?") ? "&" : "?"}view=${view}`;
}

function productImageUrl(image: string) {
  if (productMediaBaseUrl === "") {
    return image;
  }

  return `${productMediaBaseUrl}/${productMediaFileName(image)}`;
}

function normalizeProductMediaBaseUrl(baseUrl: string) {
  return baseUrl.trim().replace(/\/+$/, "");
}

function productMediaFileName(image: string) {
  const fileName = image.split("/").pop() ?? image;
  return fileName.replace(/\.svg$/i, ".png");
}

function hasProducts(data: ProductsResponse) {
  return Array.isArray(data.products) && data.products.length > 0;
}
