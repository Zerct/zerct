import { type FormEvent, useMemo, useState } from "react";
import {
  createStripeCheckout,
  formatCurrency,
  freeShippingThresholdCents,
  reserveOrder,
  shippingCents,
  type CartLine,
  type CartQuantities,
  type CheckoutFields,
  type OrderReceipt,
  type Product,
} from "./store";

type CartLineActions = {
  onAdd: (productId: string) => void;
  onDecrease: (productId: string) => void;
  onRemove: (productId: string) => void;
};

export type CartState = ReturnType<typeof useCart>;
export type CheckoutState = ReturnType<typeof useCheckout>;

export function CartDrawer({
  cart,
  checkout,
  onAdd,
  onClose,
}: {
  cart: CartState;
  checkout: CheckoutState;
  onAdd: (productId: string) => void;
  onClose: () => void;
}) {
  const orderReceipt = checkout.orderReceipt;
  const drawerContent =
    orderReceipt !== null && cart.cartLines.length === 0 ? (
      <OrderReceiptSummary orderReceipt={orderReceipt} />
    ) : (
      <>
        <CartLines cart={cart} onAdd={onAdd} />
        <CartTotals cart={cart} />
        <CheckoutForm checkout={checkout} isCartEmpty={cart.cartLines.length === 0} />
      </>
    );

  return (
    <div className="overlay-layer" role="presentation">
      <button aria-label="Close cart" className="overlay-scrim" onClick={onClose} type="button" />
      <aside aria-label="Cart and checkout" aria-modal="true" className="cart-drawer" role="dialog">
        <CartTop onClose={onClose} />
        {drawerContent}
      </aside>
    </div>
  );
}

function CartTop({ onClose }: { onClose: () => void }) {
  return (
    <div className="drawer-top">
      <h2>CART</h2>
      <button onClick={onClose} type="button">
        CLOSE
      </button>
    </div>
  );
}

function CartLines({ cart, onAdd }: { cart: CartState; onAdd: (productId: string) => void }) {
  const actions: CartLineActions = {
    onAdd,
    onDecrease: cart.decreaseQuantity,
    onRemove: cart.removeFromCart,
  };

  if (cart.cartLines.length === 0) {
    return (
      <div className="cart-lines" aria-live="polite">
        <p className="empty-cart">NO ITEMS</p>
      </div>
    );
  }

  return (
    <div className="cart-lines" aria-live="polite">
      {cart.cartLines.map((line) => (
        <CartLineRow key={line.product.id} actions={actions} line={line} />
      ))}
    </div>
  );
}

function CartLineRow({
  actions,
  line,
}: {
  actions: CartLineActions;
  line: CartLine;
}) {
  return (
    <div className="cart-line">
      <div>
        <strong>{line.product.name}</strong>
        <span>
          {line.quantity} X {formatCurrency(line.product.priceCents)}
        </span>
      </div>
      <QuantityControls actions={actions} line={line} />
    </div>
  );
}

function QuantityControls({
  actions,
  line,
}: {
  actions: CartLineActions;
  line: CartLine;
}) {
  return (
    <div className="quantity-controls">
      <button
        aria-label={`Decrease ${line.product.name}`}
        onClick={() => actions.onDecrease(line.product.id)}
        type="button"
      >
        -
      </button>
      <button
        aria-label={`Increase ${line.product.name}`}
        onClick={() => actions.onAdd(line.product.id)}
        type="button"
      >
        +
      </button>
      <button
        aria-label={`Remove ${line.product.name}`}
        onClick={() => actions.onRemove(line.product.id)}
        type="button"
      >
        REMOVE
      </button>
    </div>
  );
}

function CartTotals({ cart }: { cart: CartState }) {
  return (
    <div className="totals">
      <span>SUBTOTAL</span>
      <strong>{formatCurrency(cart.subtotalCents)}</strong>
      <span>SHIPPING</span>
      <strong>{cart.activeShippingCents === 0 ? "FREE" : formatCurrency(cart.activeShippingCents)}</strong>
      <span>TOTAL</span>
      <strong>{formatCurrency(cart.totalCents)}</strong>
    </div>
  );
}

function OrderReceiptSummary({ orderReceipt }: { orderReceipt: OrderReceipt }) {
  return (
    <section className="receipt" role="status" aria-label="Order receipt">
      <span>ORDER</span>
      <strong>{orderReceipt.id}</strong>
      <span>{orderReceipt.statusLabel ?? "RESERVED"}</span>
      <strong>{formatCurrency(orderReceipt.totalCents)}</strong>
    </section>
  );
}

function CheckoutForm({
  checkout,
  isCartEmpty,
}: {
  checkout: CheckoutState;
  isCartEmpty: boolean;
}) {
  return (
    <form className="checkout-form" onSubmit={checkout.submitOrder}>
      <ExpressCheckoutButton
        disabled={isCartEmpty}
        isSubmitting={checkout.isExpressSubmitting}
        onClick={checkout.submitExpressCheckout}
      />
      <CheckoutDivider />
      <CheckoutInput autoComplete="name" field="name" label="NAME" checkout={checkout} />
      <CheckoutInput autoComplete="email" field="email" label="EMAIL" type="email" checkout={checkout} />
      <CheckoutTextArea checkout={checkout} />
      <CheckoutErrorMessage message={checkout.checkoutError} />
      <CheckoutSubmitButton disabled={isCartEmpty || checkout.isExpressSubmitting} isSubmitting={checkout.isSubmitting} />
    </form>
  );
}

function ExpressCheckoutButton({
  disabled,
  isSubmitting,
  onClick,
}: {
  disabled: boolean;
  isSubmitting: boolean;
  onClick: () => void;
}) {
  const label = isSubmitting ? "STARTING..." : "EXPRESS CHECKOUT";
  return (
    <button className="express-checkout-button" disabled={disabled || isSubmitting} onClick={onClick} type="button">
      {label}
    </button>
  );
}

function CheckoutDivider() {
  return <span className="checkout-divider">OR</span>;
}

function CheckoutErrorMessage({ message }: { message: string }) {
  if (message === "") {
    return null;
  }

  return (
    <p className="checkout-error" role="alert">
      {message}
    </p>
  );
}

function CheckoutSubmitButton({
  disabled,
  isSubmitting,
}: {
  disabled: boolean;
  isSubmitting: boolean;
}) {
  const label = isSubmitting ? "RESERVING..." : "RESERVE ORDER";
  return (
    <button disabled={isSubmitting || disabled} type="submit">
      {label}
    </button>
  );
}

function CheckoutInput({
  autoComplete,
  checkout,
  field,
  label,
  type = "text",
}: {
  autoComplete: string;
  checkout: CheckoutState;
  field: keyof Pick<CheckoutFields, "email" | "name">;
  label: string;
  type?: string;
}) {
  return (
    <label>
      {label}
      <input
        autoComplete={autoComplete}
        name={field}
        onChange={(event) => checkout.updateCheckoutField(field, event.currentTarget.value)}
        required
        spellCheck={field === "email" ? false : undefined}
        type={type}
        value={checkout.checkoutFields[field]}
      />
    </label>
  );
}

function CheckoutTextArea({ checkout }: { checkout: CheckoutState }) {
  return (
    <label>
      ADDRESS
      <textarea
        autoComplete="shipping street-address"
        name="address"
        onChange={(event) => checkout.updateCheckoutField("address", event.currentTarget.value)}
        required
        rows={3}
        value={checkout.checkoutFields.address}
      />
    </label>
  );
}

export function useCart(products: Product[]) {
  const [cart, setCart] = useState<CartQuantities>({});
  const cartLines = useMemo(() => buildCartLines(products, cart), [cart, products]);
  const subtotalCents = useMemo(() => getSubtotalCents(cartLines), [cartLines]);
  const activeShippingCents =
    cartLines.length === 0 || subtotalCents >= freeShippingThresholdCents ? 0 : shippingCents;
  const totalCents = subtotalCents + activeShippingCents;
  const totalItems = cartLines.reduce((total, line) => total + line.quantity, 0);

  return {
    activeShippingCents,
    addToCart: (productId: string) => updateCartQuantity(setCart, productId, 1),
    cartLines,
    clearCart: () => setCart({}),
    decreaseQuantity: (productId: string) => updateCartQuantity(setCart, productId, -1),
    removeFromCart: (productId: string) => removeCartLine(setCart, productId),
    subtotalCents,
    totalCents,
    totalItems,
  };
}

export function useCheckout(cartLines: CartLine[], totalCents: number, clearCart: () => void) {
  const [checkoutFields, setCheckoutFields] = useState<CheckoutFields>({
    address: "",
    email: "",
    name: "",
  });
  const [checkoutError, setCheckoutError] = useState("");
  const [isExpressSubmitting, setIsExpressSubmitting] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [orderReceipt, setOrderReceipt] = useState<OrderReceipt | null>(null);

  async function submitOrder(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setCheckoutError("");
    setIsSubmitting(true);
    try {
      const orderId = await reserveOrder(checkoutFields, cartLines, totalCents);
      setOrderReceipt({ id: orderId, totalCents });
      setCheckoutFields({ address: "", email: "", name: "" });
      clearCart();
    } catch {
      setCheckoutError("ORDER FAILED");
    } finally {
      setIsSubmitting(false);
    }
  }

  async function submitExpressCheckout() {
    setCheckoutError("");
    setIsExpressSubmitting(true);
    try {
      const result = await createStripeCheckout(checkoutFields, cartLines, totalCents);
      if (result.mode === "stripe") {
        window.location.assign(result.checkoutUrl);
        return;
      }
      setOrderReceipt({ id: result.orderId, statusLabel: "STRIPE DEMO", totalCents });
      setCheckoutFields({ address: "", email: "", name: "" });
      clearCart();
    } catch {
      setCheckoutError("EXPRESS CHECKOUT FAILED");
    } finally {
      setIsExpressSubmitting(false);
    }
  }

  return {
    checkoutError,
    checkoutFields,
    clearReceipt: () => {
      setCheckoutError("");
      setOrderReceipt(null);
    },
    isExpressSubmitting,
    isSubmitting,
    orderReceipt,
    submitExpressCheckout,
    submitOrder,
    updateCheckoutField: (field: keyof CheckoutFields, value: string) =>
      setCheckoutFields((currentFields) => ({ ...currentFields, [field]: value })),
  };
}

function buildCartLines(products: Product[], cart: CartQuantities) {
  return products
    .map((product) => ({
      product,
      quantity: cart[product.id] ?? 0,
    }))
    .filter((line) => line.quantity > 0);
}

function getSubtotalCents(cartLines: CartLine[]) {
  return cartLines.reduce((total, line) => total + line.product.priceCents * line.quantity, 0);
}

function updateCartQuantity(
  setCart: (update: (currentCart: CartQuantities) => CartQuantities) => void,
  productId: string,
  delta: number,
) {
  setCart((currentCart) => {
    const nextQuantity = (currentCart[productId] ?? 0) + delta;
    const nextCart = { ...currentCart };
    if (nextQuantity <= 0) {
      delete nextCart[productId];
    } else {
      nextCart[productId] = nextQuantity;
    }
    return nextCart;
  });
}

function removeCartLine(
  setCart: (update: (currentCart: CartQuantities) => CartQuantities) => void,
  productId: string,
) {
  setCart((currentCart) => {
    const nextCart = { ...currentCart };
    delete nextCart[productId];
    return nextCart;
  });
}
