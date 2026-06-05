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
  address: string;
  email: string;
  name: string;
};

export type OrderReceipt = {
  id: string;
  statusLabel?: string;
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

type ProductCatalog = {
  products: Product[];
};

type ProductsResponse = {
  products: Product[];
};

type CheckoutApiResponse = {
  checkoutUrl?: string;
  error?: string;
  message?: string;
  mode?: "demo" | "stripe";
  orderId?: string;
};

type CartApiResponse = {
  data: CheckoutApiResponse;
  ok: boolean;
};

const productCatalog = productCatalogJson as ProductCatalog;

export const fallbackProducts: Product[] = productCatalog.products.map((product) =>
  shapeProduct(product.id, product.name, product.image, product.priceCents, product.inventory, product.categories),
);

export async function fetchProducts() {
  const response = await fetch(`${apiBaseUrl}/products`);
  if (!response.ok) {
    throw new Error(`Product API returned ${response.status}`);
  }

  const data = (await response.json()) as ProductsResponse;
  if (!hasProducts(data)) {
    throw new Error("Product API returned an empty catalog");
  }

  return data.products;
}

export async function reserveOrder(checkoutFields: CheckoutFields, cartLines: CartLine[], totalCents: number) {
  const response = await postCartRequest("orders", checkoutFields, cartLines, totalCents);
  if (!response.ok || typeof response.data.orderId !== "string") {
    throw new Error("Order API did not return a receipt");
  }
  return response.data.orderId;
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

function shapeProduct(
  id: string,
  name: string,
  image: string,
  priceCents: number,
  inventory: number,
  categories: readonly Category[],
): Product {
  return { categories, id, image, inventory, name, priceCents };
}

function hasProducts(data: ProductsResponse) {
  return Array.isArray(data.products) && data.products.length > 0;
}
