import { useEffect, useState } from "react";
import { formatCurrency, productSizes, type Category, type Product } from "./store";
import { getProductTransitionStyle } from "./transitions";

type ProductGridProps = {
  onAdd: (productId: string, selectedSize?: string) => void;
  onViewProduct: (product: Product) => void;
  products: Product[];
  selectedCategory: Category;
  selectedProduct: Product | null;
};

type ProductFocusViewProps = {
  onAdd: (productId: string, selectedSize?: string) => void;
  onViewProduct: (product: Product) => void;
  product: Product;
  products: Product[];
  selectedIndex: number;
};

export function ProductGrid({
  onAdd,
  onViewProduct,
  products,
  selectedCategory,
  selectedProduct,
}: ProductGridProps) {
  const productFocus = getProductFocus(products, selectedProduct);

  if (productFocus !== null) {
    return (
      <ProductFocusView
        onAdd={onAdd}
        onViewProduct={onViewProduct}
        product={productFocus.product}
        products={products}
        selectedIndex={productFocus.index}
      />
    );
  }

  return (
    <section className="product-grid" aria-label={`${selectedCategory} products`}>
      {products.map((product) => (
        <ProductTile key={product.id} onViewProduct={onViewProduct} product={product} />
      ))}
    </section>
  );
}

function ProductTile({ onViewProduct, product }: { onViewProduct: (product: Product) => void; product: Product }) {
  return <ProductButton className="product-tile" onViewProduct={onViewProduct} product={product} />;
}

function ProductButton({
  ariaCurrent,
  className,
  loading = "lazy",
  onViewProduct,
  product,
}: {
  ariaCurrent?: "true" | undefined;
  className: string;
  loading?: "eager" | "lazy";
  onViewProduct: (product: Product) => void;
  product: Product;
}) {
  return (
    <button
      aria-current={ariaCurrent}
      aria-label={`View ${product.name}`}
      className={className}
      onClick={() => onViewProduct(product)}
      type="button"
    >
      <ProductFigure loading={loading} product={product} />
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
  onAdd,
  onViewProduct,
  product,
  products,
  selectedIndex,
}: ProductFocusViewProps) {
  const [isSizePickerOpen, setIsSizePickerOpen] = useState(false);
  const focusProducts = getFocusProducts(products, selectedIndex);

  useEffect(() => {
    setIsSizePickerOpen(false);
  }, [product.id]);

  return (
    <section aria-label={`${product.name} details`} className="product-focus">
      <div className="product-focus-stage">
        <div className={focusProducts.length === 1 ? "product-focus-rail single-product" : "product-focus-rail"}>
          {focusProducts.map((focusProduct) => (
            <ProductButton
              ariaCurrent={getFocusAriaCurrent(focusProduct.id === product.id)}
              className={getFocusTileClass(focusProduct.id === product.id)}
              key={focusProduct.id}
              loading={getFocusImageLoading(focusProduct.id === product.id)}
              onViewProduct={onViewProduct}
              product={focusProduct}
            />
          ))}
        </div>
        <ProductDots />
        {isSizePickerOpen ? (
          <SizePicker
            onAdd={(selectedSize) => {
              onAdd(product.id, selectedSize);
              setIsSizePickerOpen(false);
            }}
            onClose={() => setIsSizePickerOpen(false)}
            product={product}
          />
        ) : (
          <div className="product-detail-meta">
            <strong>{product.name}</strong>
            <span>{formatCurrency(product.priceCents)}</span>
            <button aria-label={`Select size for ${product.name}`} onClick={() => setIsSizePickerOpen(true)} type="button">
              +
            </button>
          </div>
        )}
      </div>
    </section>
  );
}

function ProductDots() {
  return (
    <div aria-hidden="true" className="product-dots">
      {Array.from({ length: 8 }, (_value, index) => (
        <span className={index === 0 ? "active" : ""} key={index} />
      ))}
    </div>
  );
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
      <span>{formatCurrency(product.priceCents)}</span>
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
    </div>
  );
}

function getProductFocus(products: Product[], selectedProduct: Product | null) {
  if (selectedProduct === null) {
    return null;
  }

  const selectedIndex = products.findIndex((product) => product.id === selectedProduct.id);
  return selectedIndex >= 0 ? { index: selectedIndex, product: selectedProduct } : null;
}

function getFocusProducts(products: Product[], selectedIndex: number) {
  const selectedProduct = products[selectedIndex];
  if (selectedProduct === undefined) {
    return [];
  }

  return buildFocusProducts(products, selectedIndex, selectedProduct);
}

function buildFocusProducts(products: Product[], selectedIndex: number, selectedProduct: Product) {
  if (products.length === 1) {
    return [selectedProduct];
  }

  const previousProduct = getWrappedProduct(products, selectedIndex - 1, selectedProduct);
  const nextProduct = getWrappedProduct(products, selectedIndex + 1, selectedProduct);
  return [previousProduct, selectedProduct, nextProduct];
}

function getWrappedProduct(products: Product[], index: number, fallbackProduct: Product) {
  const wrappedIndex = (index + products.length) % products.length;
  return products[wrappedIndex] ?? fallbackProduct;
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
