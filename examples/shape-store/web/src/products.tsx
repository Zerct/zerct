import { useCallback, useEffect, useState } from "react";
import { formatCurrency, productSizes, type Category, type Product } from "./store";
import { getProductTransitionStyle } from "./transitions";

type ProductGridProps = {
  isKeyboardEnabled: boolean;
  onAdd: (productId: string, selectedSize?: string) => void;
  onProductClose: () => void;
  onViewProduct: (product: Product) => void;
  products: Product[];
  selectedCategory: Category;
  selectedProduct: Product | null;
};

type ProductFocusViewProps = {
  isKeyboardEnabled: boolean;
  onAdd: (productId: string, selectedSize?: string) => void;
  onProductClose: () => void;
  product: Product;
};

type GalleryPosition = "previous" | "active" | "next";

type GalleryItem = {
  image: string;
  imageIndex: number;
  position: GalleryPosition;
};

type ProductKeyboardKey = "ArrowLeft" | "ArrowRight" | "Escape";

type ProductDetailCopy = {
  categoryLabel: string;
  lines: readonly string[];
  status: string;
};

const productCategoryLabels = {
  ACCESSORIES: "SHAPE ACCESSORY",
  FOOTWEAR: "SHAPE FOOTWEAR",
  SLIDES: "SHAPE SLIDE",
} as const satisfies Partial<Record<Category, string>>;
const productCategoryPriority = ["SLIDES", "FOOTWEAR", "ACCESSORIES"] as const satisfies readonly Category[];

export function ProductGrid({
  isKeyboardEnabled,
  onAdd,
  onProductClose,
  onViewProduct,
  products,
  selectedCategory,
  selectedProduct,
}: ProductGridProps) {
  const productFocus = getProductFocus(products, selectedProduct);

  if (productFocus !== null) {
    return (
      <ProductFocusView
        isKeyboardEnabled={isKeyboardEnabled}
        onAdd={onAdd}
        onProductClose={onProductClose}
        product={productFocus.product}
      />
    );
  }

  return (
    <section className="product-grid" data-testid="product-grid" aria-label={`${selectedCategory} products`}>
      {products.map((product) => (
        <ProductTile key={product.id} onViewProduct={onViewProduct} product={product} />
      ))}
    </section>
  );
}

function ProductTile({ onViewProduct, product }: { onViewProduct: (product: Product) => void; product: Product }) {
  return (
    <button
      aria-label={`View ${product.name}`}
      className="product-tile"
      data-testid="product-card"
      onClick={() => onViewProduct(product)}
      type="button"
    >
      <ProductFigure product={product} />
    </button>
  );
}

function ProductFigure({ loading = "lazy", product }: { loading?: "eager" | "lazy"; product: Product }) {
  return (
    <>
      <img
        alt={product.name}
        height="512"
        loading={loading}
        src={product.image}
        style={getProductTransitionStyle(product.id)}
        width="512"
      />
      <span>{product.name}</span>
    </>
  );
}

function ProductFocusView({
  isKeyboardEnabled,
  onAdd,
  onProductClose,
  product,
}: ProductFocusViewProps) {
  const gallery = useProductGallery(product);
  const [isSizePickerOpen, setIsSizePickerOpen] = useState(false);
  const addFeedback = useAddFeedback(product.id);

  useEffect(() => {
    setIsSizePickerOpen(false);
  }, [product.id]);

  useProductKeyboardControls({
    hasMultipleImages: gallery.hasMultipleImages,
    isEnabled: isKeyboardEnabled,
    onNext: gallery.showNext,
    onProductClose,
    onPrevious: gallery.showPrevious,
  });

  function addSelectedSize(selectedSize: string) {
    onAdd(product.id, selectedSize);
    setIsSizePickerOpen(false);
    addFeedback.show();
  }

  return (
    <section aria-label={`${product.name} details`} className="product-focus">
      <div className={productFocusStageClass(isSizePickerOpen)}>
        <div
          className={gallery.hasMultipleImages ? "product-focus-rail" : "product-focus-rail single-image"}
          key={`${product.id}-${gallery.selectedImageIndex}`}
        >
          {gallery.items.map((item) => (
            <GalleryImageButton
              isActive={item.position === "active"}
              item={item}
              key={`${product.id}-${item.imageIndex}-${item.position}`}
              onSelect={() => gallery.selectImage(item.imageIndex)}
              product={product}
            />
          ))}
        </div>
        <GalleryArrows gallery={gallery} productName={product.name} />
        <ProductDots
          imageCount={gallery.images.length}
          onSelect={gallery.selectImage}
          selectedImageIndex={gallery.selectedImageIndex}
        />
        <ProductPurchaseControls
          addFeedbackKey={addFeedback.key}
          isSizePickerOpen={isSizePickerOpen}
          onAdd={addSelectedSize}
          onSizePickerClose={() => setIsSizePickerOpen(false)}
          onSizePickerOpen={() => setIsSizePickerOpen(true)}
          product={product}
        />
      </div>
    </section>
  );
}

function useProductGallery(product: Product) {
  const images = getProductGalleryImages(product);
  const [selectedImageIndex, setSelectedImageIndex] = useState(0);
  const items = getGalleryItems(images, selectedImageIndex);
  const hasMultipleImages = images.length > 1;
  const showNext = useCallback(
    () => setSelectedImageIndex((currentIndex) => wrappedGalleryIndex(images, currentIndex + 1)),
    [images],
  );
  const showPrevious = useCallback(
    () => setSelectedImageIndex((currentIndex) => wrappedGalleryIndex(images, currentIndex - 1)),
    [images],
  );

  useEffect(() => {
    setSelectedImageIndex(0);
  }, [product.id]);

  return {
    hasMultipleImages,
    images,
    items,
    selectedImageIndex,
    selectImage: setSelectedImageIndex,
    showNext,
    showPrevious,
  };
}

function useAddFeedback(productId: string) {
  const [key, setKey] = useState(0);

  useEffect(() => {
    setKey(0);
  }, [productId]);

  useEffect(() => {
    if (key === 0) {
      return undefined;
    }

    const timeout = window.setTimeout(() => setKey(0), 1100);
    return () => window.clearTimeout(timeout);
  }, [key]);

  return {
    key,
    show: () => setKey(Date.now()),
  };
}

function useProductKeyboardControls({
  hasMultipleImages,
  isEnabled,
  onNext,
  onProductClose,
  onPrevious,
}: {
  hasMultipleImages: boolean;
  isEnabled: boolean;
  onNext: () => void;
  onProductClose: () => void;
  onPrevious: () => void;
}) {
  useEffect(() => {
    if (!isEnabled) {
      return undefined;
    }

    const keyboardHandlers = {
      Escape: onProductClose,
      ...(hasMultipleImages ? { ArrowLeft: onPrevious, ArrowRight: onNext } : {}),
    } satisfies Partial<Record<ProductKeyboardKey, () => void>>;

    function handleKeyDown(event: KeyboardEvent) {
      const handler = keyboardHandlers[event.key as ProductKeyboardKey];
      if (handler === undefined) {
        return;
      }

      event.preventDefault();
      handler();
    }

    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, [hasMultipleImages, isEnabled, onNext, onProductClose, onPrevious]);
}

function GalleryImageButton({
  isActive,
  item,
  onSelect,
  product,
}: {
  isActive: boolean;
  item: GalleryItem;
  onSelect: () => void;
  product: Product;
}) {
  return (
    <button
      aria-current={getFocusAriaCurrent(isActive)}
      aria-label={`View ${product.name} image ${item.imageIndex + 1}`}
      className={getFocusTileClass(isActive)}
      onClick={onSelect}
      type="button"
    >
      <img
        alt={product.name}
        height="512"
        loading={getFocusImageLoading(isActive)}
        src={item.image}
        style={isActive && item.imageIndex === 0 ? getProductTransitionStyle(product.id) : undefined}
        width="512"
      />
      <span>{product.name}</span>
    </button>
  );
}

function GalleryArrow({
  direction,
  onClick,
  productName,
}: {
  direction: "previous" | "next";
  onClick: () => void;
  productName: string;
}) {
  const label = direction === "previous" ? `Previous image for ${productName}` : `Next image for ${productName}`;
  return (
    <button aria-label={label} className={`gallery-arrow ${direction}`} onClick={onClick} type="button">
      <span aria-hidden="true" />
    </button>
  );
}

function GalleryArrows({
  gallery,
  productName,
}: {
  gallery: ReturnType<typeof useProductGallery>;
  productName: string;
}) {
  if (!gallery.hasMultipleImages) {
    return null;
  }

  return (
    <>
      <GalleryArrow direction="previous" onClick={gallery.showPrevious} productName={productName} />
      <GalleryArrow direction="next" onClick={gallery.showNext} productName={productName} />
    </>
  );
}

function ProductDots({
  imageCount,
  onSelect,
  selectedImageIndex,
}: {
  imageCount: number;
  onSelect: (index: number) => void;
  selectedImageIndex: number;
}) {
  if (imageCount <= 1) {
    return null;
  }

  return (
    <div className="product-dots" role="group" aria-label="Product images">
      {Array.from({ length: imageCount }, (_value, index) => (
        <button
          aria-label={`Show image ${index + 1}`}
          aria-pressed={index === selectedImageIndex}
          className={index === selectedImageIndex ? "active" : ""}
          key={index}
          onClick={() => onSelect(index)}
          type="button"
        />
      ))}
    </div>
  );
}

function ProductDetailMeta({
  addFeedbackKey,
  onSizePickerOpen,
  product,
}: {
  addFeedbackKey: number;
  onSizePickerOpen: () => void;
  product: Product;
}) {
  const detailCopy = productDetailCopy(product);
  return (
    <div className="product-detail-meta">
      <strong>{product.name}</strong>
      <ProductPriceStatus detailCopy={detailCopy} product={product} />
      <button aria-label={`Select size for ${product.name}`} onClick={onSizePickerOpen} type="button">
        +
      </button>
      {addFeedbackKey > 0 ? (
        <p className="add-feedback" key={addFeedbackKey} role="status">
          ADDED TO BAG
        </p>
      ) : null}
    </div>
  );
}

function ProductPurchaseControls({
  addFeedbackKey,
  isSizePickerOpen,
  onAdd,
  onSizePickerClose,
  onSizePickerOpen,
  product,
}: {
  addFeedbackKey: number;
  isSizePickerOpen: boolean;
  onAdd: (selectedSize: string) => void;
  onSizePickerClose: () => void;
  onSizePickerOpen: () => void;
  product: Product;
}) {
  if (isSizePickerOpen) {
    return <SizePicker onAdd={onAdd} onClose={onSizePickerClose} product={product} />;
  }

  return <ProductDetailMeta addFeedbackKey={addFeedbackKey} onSizePickerOpen={onSizePickerOpen} product={product} />;
}

function SizePicker({
  onAdd,
  onClose,
  product,
}: {
  onAdd: (selectedSize: string) => void;
  onClose: () => void;
  product: Product;
}) {
  const detailCopy = productDetailCopy(product);
  return (
    <div className="size-picker" role="group" aria-label={`Select size for ${product.name}`}>
      <div className="size-picker-top">
        <button aria-label="Size help" type="button">
          ?
        </button>
        <strong>SELECT SIZE</strong>
        <button aria-label="Close size picker" onClick={onClose} type="button">
          X
        </button>
      </div>
      <ProductPriceStatus detailCopy={detailCopy} product={product} />
      <div className="size-grid">
        {productSizes.map((size) => (
          <button key={size} onClick={() => onAdd(size)} type="button">
            {size}
          </button>
        ))}
      </div>
      <button className="size-picker-info" type="button">
        INFORMATION
      </button>
      <ProductInformation detailCopy={detailCopy} />
    </div>
  );
}

function ProductPriceStatus({
  detailCopy,
  product,
}: {
  detailCopy: ProductDetailCopy;
  product: Product;
}) {
  return (
    <span className="product-price-status">
      <span>{formatCurrency(product.priceCents)}</span>
      <span>{detailCopy.status}</span>
    </span>
  );
}

function ProductInformation({ detailCopy }: { detailCopy: ProductDetailCopy }) {
  return (
    <dl className="product-information" aria-label="Product information">
      <dt>{detailCopy.categoryLabel}</dt>
      {detailCopy.lines.map((line) => (
        <dd key={line}>{line}</dd>
      ))}
    </dl>
  );
}

function getProductFocus(products: Product[], selectedProduct: Product | null) {
  if (selectedProduct === null) {
    return null;
  }

  const productExists = products.some((product) => product.id === selectedProduct.id);
  return productExists ? { product: selectedProduct } : null;
}

function getProductGalleryImages(product: Product) {
  return product.galleryImages.length > 0 ? product.galleryImages : [product.image];
}

function productDetailCopy(product: Product): ProductDetailCopy {
  return {
    categoryLabel: productCategoryLabel(product),
    lines: [
      "100% SOLID BLACK VECTOR",
      "FITS TRUE TO SIZE",
      "SHIPS 3 TO 5 BUSINESS DAYS",
    ],
    status: product.inventory <= 12 ? "LIMITED RUN" : "RESTOCKS IN 4 WEEKS",
  };
}

function productCategoryLabel(product: Product) {
  const category = productCategoryPriority.find((candidate) => product.categories.includes(candidate));
  return category === undefined ? "SHAPE PIECE" : productCategoryLabels[category];
}

function getGalleryItems(images: readonly string[], selectedIndex: number): GalleryItem[] {
  if (images.length === 1) {
    return [{ image: images[0] ?? "", imageIndex: 0, position: "active" }];
  }

  return [
    galleryItem(images, selectedIndex - 1, "previous"),
    galleryItem(images, selectedIndex, "active"),
    galleryItem(images, selectedIndex + 1, "next"),
  ];
}

function galleryItem(images: readonly string[], index: number, position: GalleryPosition): GalleryItem {
  const imageIndex = wrappedGalleryIndex(images, index);
  return {
    image: images[imageIndex] ?? "",
    imageIndex,
    position,
  };
}

function wrappedGalleryIndex(images: readonly string[], index: number) {
  return (index + images.length) % images.length;
}

function getFocusAriaCurrent(isActive: boolean): "true" | undefined {
  return isActive ? "true" : undefined;
}

function getFocusImageLoading(isActive: boolean): "eager" | "lazy" {
  return isActive ? "eager" : "lazy";
}

function getFocusTileClass(isActive: boolean) {
  return isActive ? "product-focus-tile active" : "product-focus-tile";
}

function productFocusStageClass(isSizePickerOpen: boolean) {
  return isSizePickerOpen ? "product-focus-stage size-picker-open" : "product-focus-stage";
}
