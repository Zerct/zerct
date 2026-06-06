import { useEffect, useMemo, useState } from "react";
import { CartDrawer, formatCartItemCount, useCart, useCheckout, type CartState, type CheckoutState } from "./cart";
import { ProductGrid } from "./products";
import {
  categoryTabs,
  fallbackProducts,
  fetchProducts,
  type Category,
  type LoadState,
  type Product,
} from "./store";
import { transitionStoreState } from "./transitions";

export function App() {
  const { loadState, products } = useProducts();
  const cart = useCart(products);
  const checkout = useCheckout(cart.cartLines, cart.totalCents, cart.clearCart);
  const [isCartOpen, setIsCartOpen] = useState(false);
  const [isMenuOpen, setIsMenuOpen] = useState(false);
  const [selectedProduct, setSelectedProduct] = useState<Product | null>(null);
  const [selectedCategory, setSelectedCategory] = useState<Category>("NEW");
  const visibleProducts = useVisibleProducts(products, selectedCategory);

  function addProduct(productId: string, selectedSize?: string) {
    transitionStoreState(() => {
      cart.addToCart(productId, selectedSize);
      checkout.clearReceipt();
    });
  }

  function closeProduct() {
    transitionStoreState(() => setSelectedProduct(null));
  }

  function viewProduct(product: Product) {
    transitionStoreState(() => {
      setIsMenuOpen(false);
      setSelectedProduct(product);
    });
  }

  return (
    <main>
      <h1 className="sr-only">Shape Store</h1>
      <SiteHeader
        isProductMode={selectedProduct !== null}
        onCartOpen={() => setIsCartOpen(true)}
        onCategorySelect={setSelectedCategory}
        onMenuOpen={() => setIsMenuOpen(true)}
        onProductClose={closeProduct}
        selectedCategory={selectedCategory}
        totalItems={cart.totalItems}
      />
      <ApiStateBanner loadState={loadState} />
      <ProductGrid
        isKeyboardEnabled={!isCartOpen && !isMenuOpen}
        onAdd={addProduct}
        onProductClose={closeProduct}
        products={visibleProducts}
        selectedCategory={selectedCategory}
        selectedProduct={selectedProduct}
        onViewProduct={viewProduct}
      />
      <OverlayLayers
        cart={cart}
        checkout={checkout}
        isCartOpen={isCartOpen}
        isMenuOpen={isMenuOpen}
        onAdd={addProduct}
        onCartClose={() => setIsCartOpen(false)}
        onMenuClose={() => setIsMenuOpen(false)}
      />
      <ConditionalFooter selectedProduct={selectedProduct} />
    </main>
  );
}

function ApiStateBanner({ loadState }: { loadState: LoadState }) {
  return loadState === "error" ? <p className="api-state">API FALLBACK</p> : null;
}

function OverlayLayers({
  cart,
  checkout,
  isCartOpen,
  isMenuOpen,
  onAdd,
  onCartClose,
  onMenuClose,
}: {
  cart: CartState;
  checkout: CheckoutState;
  isCartOpen: boolean;
  isMenuOpen: boolean;
  onAdd: (productId: string, selectedSize?: string) => void;
  onCartClose: () => void;
  onMenuClose: () => void;
}) {
  return (
    <>
      {isMenuOpen ? <MenuDrawer onClose={onMenuClose} /> : null}
      {isCartOpen ? <CartDrawer cart={cart} checkout={checkout} onAdd={onAdd} onClose={onCartClose} /> : null}
    </>
  );
}

function ConditionalFooter({ selectedProduct }: { selectedProduct: Product | null }) {
  return selectedProduct === null ? <SiteFooter /> : null;
}

function SiteHeader({
  isProductMode,
  onCartOpen,
  onCategorySelect,
  onMenuOpen,
  onProductClose,
  selectedCategory,
  totalItems,
}: {
  isProductMode: boolean;
  onCartOpen: () => void;
  onCategorySelect: (category: Category) => void;
  onMenuOpen: () => void;
  onProductClose: () => void;
  selectedCategory: Category;
  totalItems: number;
}) {
  return (
    <header className={isProductMode ? "site-header product-mode" : "site-header"}>
      <div className="header-tools">
        <HeaderStartButton isProductMode={isProductMode} onMenuOpen={onMenuOpen} onProductClose={onProductClose} />
      </div>
      <HeaderNavigation
        isProductMode={isProductMode}
        onCategorySelect={onCategorySelect}
        selectedCategory={selectedCategory}
      />
      <button
        aria-label={`Open cart with ${formatCartItemCount(totalItems)}`}
        className="cart-trigger"
        onClick={onCartOpen}
        type="button"
      >
        <span aria-hidden="true" className="cart-icon" />
        <span className="cart-count" key={totalItems}>
          {totalItems}
        </span>
      </button>
    </header>
  );
}

function HeaderStartButton({
  isProductMode,
  onMenuOpen,
  onProductClose,
}: {
  isProductMode: boolean;
  onMenuOpen: () => void;
  onProductClose: () => void;
}) {
  if (isProductMode) {
    return (
      <button aria-label="Back to products" className="back-trigger header-back-trigger" onClick={onProductClose} type="button">
        <span aria-hidden="true" />
      </button>
    );
  }

  return (
    <button aria-label="Open menu" className="menu-trigger" onClick={onMenuOpen} type="button">
      <span aria-hidden="true" className="menu-icon" />
    </button>
  );
}

function HeaderNavigation({
  isProductMode,
  onCategorySelect,
  selectedCategory,
}: {
  isProductMode: boolean;
  onCategorySelect: (category: Category) => void;
  selectedCategory: Category;
}) {
  if (isProductMode) {
    return <div aria-hidden="true" />;
  }

  return (
    <nav aria-label="Product categories">
      {categoryTabs.map((category) => (
        <button
          className={selectedCategory === category ? "active" : ""}
          key={category}
          onClick={() => onCategorySelect(category)}
          type="button"
        >
          {category}
        </button>
      ))}
    </nav>
  );
}

function MenuDrawer({ onClose }: { onClose: () => void }) {
  return (
    <div className="overlay-layer" role="presentation">
      <button aria-label="Close menu" className="overlay-scrim" onClick={onClose} type="button" />
      <aside aria-label="Menu" aria-modal="true" className="menu-drawer" role="dialog">
        <div className="drawer-top">
          <h2>MENU</h2>
          <button onClick={onClose} type="button">
            CLOSE
          </button>
        </div>
        <nav aria-label="Store links" className="menu-links">
          <a href="mailto:example@example.com">CONTACT</a>
          <a href="#terms">TERMS</a>
          <a href="#privacy">PRIVACY</a>
          <a href="#orders">ORDER STATUS</a>
        </nav>
      </aside>
    </div>
  );
}

function SiteFooter() {
  return (
    <footer className="site-footer">
      <a href="mailto:example@example.com">Contact</a>
      <a href="#terms">Terms</a>
      <a href="#privacy">Privacy</a>
      <a href="#accessibility">Accessibility</a>
      <a href="#orders">Order Status</a>
    </footer>
  );
}

function useProducts() {
  const [loadState, setLoadState] = useState<LoadState>("loading");
  const [products, setProducts] = useState<Product[]>(fallbackProducts);

  useEffect(() => {
    let shouldIgnore = false;
    void fetchProducts()
      .then((remoteProducts) => {
        if (!shouldIgnore) {
          setProducts(remoteProducts);
          setLoadState("ready");
        }
      })
      .catch(() => {
        if (!shouldIgnore) {
          setProducts(fallbackProducts);
          setLoadState("error");
        }
      });

    return () => {
      shouldIgnore = true;
    };
  }, []);

  return { loadState, products };
}

function useVisibleProducts(products: Product[], selectedCategory: Category) {
  return useMemo(
    () =>
      selectedCategory === "NEW"
        ? products
        : products.filter((product) => product.categories.includes(selectedCategory)),
    [products, selectedCategory],
  );
}
