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
  onAdd: (productId: string, selectedSize?: string) => void;
  onClose: () => void;
}) {
  if (checkout.orderReceipt !== null && cart.cartLines.length === 0) {
    return <ReceiptCartDrawer onClose={onClose} orderReceipt={checkout.orderReceipt} />;
  }

  return <CheckoutCartDrawer cart={cart} checkout={checkout} onAdd={onAdd} onClose={onClose} />;
}

function ReceiptCartDrawer({ onClose, orderReceipt }: { onClose: () => void; orderReceipt: OrderReceipt }) {
  return (
    <div className="overlay-layer" role="presentation">
      <button aria-label="Close cart" className="overlay-scrim" onClick={onClose} type="button" />
      <aside aria-label="Cart and checkout" aria-modal="true" className="cart-drawer receipt-drawer" role="dialog">
        <OrderReceiptSummary onClose={onClose} orderReceipt={orderReceipt} />
      </aside>
    </div>
  );
}

function CheckoutCartDrawer({
  cart,
  checkout,
  onAdd,
  onClose,
}: {
  cart: CartState;
  checkout: CheckoutState;
  onAdd: (productId: string, selectedSize?: string) => void;
  onClose: () => void;
}) {
  return (
    <div className="overlay-layer" role="presentation">
      <button aria-label="Close cart" className="overlay-scrim" onClick={onClose} type="button" />
      <aside aria-label="Cart and checkout" aria-modal="true" className="cart-drawer" role="dialog">
        <CartTop onClose={onClose} totalItems={cart.totalItems} />
        <CheckoutLayout cart={cart} checkout={checkout} isCartEmpty={cart.cartLines.length === 0} onAdd={onAdd} />
      </aside>
    </div>
  );
}

function CheckoutLayout({
  cart,
  checkout,
  isCartEmpty,
  onAdd,
}: {
  cart: CartState;
  checkout: CheckoutState;
  isCartEmpty: boolean;
  onAdd: (productId: string) => void;
}) {
  return (
    <div className="checkout-layout">
      <div className="checkout-primary">
        <CheckoutForm checkout={checkout} isCartEmpty={isCartEmpty} />
      </div>
      <aside className="checkout-summary-panel" aria-label="Order summary">
        <OrderSummary cart={cart} onAdd={onAdd} />
        <CartTotals cart={cart} />
      </aside>
    </div>
  );
}

function CartTop({ onClose, totalItems }: { onClose: () => void; totalItems: number }) {
  return (
    <div className="checkout-top">
      <button aria-label="Back to products" className="back-trigger" onClick={onClose} type="button">
        <span aria-hidden="true" />
      </button>
      <div className="checkout-bag" aria-label={`Cart has ${totalItems} items`}>
        <span>SHOPPING BAG</span>
        <span>{totalItems}</span>
        <span aria-hidden="true" className="cart-icon" />
      </div>
    </div>
  );
}

function OrderSummary({ cart, onAdd }: { cart: CartState; onAdd: (productId: string) => void }) {
  const actions: CartLineActions = {
    onAdd,
    onDecrease: cart.decreaseQuantity,
  };

  if (cart.cartLines.length === 0) {
    return (
      <section className="order-summary" aria-live="polite">
        <h2>ORDER SUMMARY</h2>
        <p className="empty-cart">NO ITEMS</p>
      </section>
    );
  }

  return (
    <section className="order-summary" aria-live="polite">
      <h2>ORDER SUMMARY</h2>
      {cart.cartLines.map((line) => (
        <CartLineRow key={line.product.id} actions={actions} line={line} />
      ))}
    </section>
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
      <img alt="" aria-hidden="true" height="96" src={line.product.image} width="96" />
      <div className="cart-line-meta">
        <strong>{line.product.name}</strong>
        <span>SIZE</span>
        <span>{line.selectedSize ?? "-"}</span>
        <span>QTY</span>
      </div>
      <strong className="cart-line-price">{formatCurrency(line.product.priceCents)}</strong>
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
        aria-label={`Increase ${line.product.name}`}
        onClick={() => actions.onAdd(line.product.id)}
        type="button"
      >
        +
      </button>
      <span>{line.quantity}</span>
      <button
        aria-label={`Decrease ${line.product.name}`}
        onClick={() => actions.onDecrease(line.product.id)}
        type="button"
      >
        -
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
      <strong>CALCULATED AT NEXT STEP</strong>
      <span>TAXES</span>
      <strong>$0.00</strong>
      <span>TOTAL</span>
      <strong>{formatCurrency(cart.totalCents)}</strong>
    </div>
  );
}

function OrderReceiptSummary({ onClose, orderReceipt }: { onClose: () => void; orderReceipt: OrderReceipt }) {
  return (
    <section className="receipt checkout-success" role="status" aria-label="Order receipt">
      <h2>ORDER CONFIRMED</h2>
      <dl className="receipt-grid">
        <div className="receipt-row">
          <dt>ORDER</dt>
          <dd>{orderReceipt.id}</dd>
        </div>
        <div className="receipt-row">
          <dt>TOTAL</dt>
          <dd>{formatCurrency(orderReceipt.totalCents)}</dd>
        </div>
      </dl>
      <button className="primary-action" onClick={onClose} type="button">
        CONTINUE SHOPPING
      </button>
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
      <button className="discount-code-button" type="button">
        DISCOUNT CODE
      </button>
      <ExpressCheckoutButtons
        disabled={isCartEmpty}
        isSubmitting={checkout.isExpressSubmitting}
        onClick={checkout.submitExpressCheckout}
      />
      <CheckoutDivider />
      <h2>CONTACT INFORMATION</h2>
      <CheckoutInput autoComplete="email" field="email" label="EMAIL ADDRESS" type="email" checkout={checkout} />
      <h2>SHIPPING ADDRESS</h2>
      <div className="checkout-field-row">
        <CheckoutInput autoComplete="given-name" field="firstName" label="FIRST NAME" checkout={checkout} />
        <CheckoutInput autoComplete="family-name" field="lastName" label="LAST NAME" checkout={checkout} />
      </div>
      <CheckoutInput
        autoComplete="shipping street-address"
        field="address"
        label="ADDRESS"
        placeholder="START TYPING YOUR ADDRESS..."
        checkout={checkout}
      />
      <CheckoutInput
        autoComplete="shipping address-line2"
        field="apartment"
        label="APARTMENT, SUITE, UNIT, FLOOR, ETC."
        placeholder="OPTIONAL"
        required={false}
        checkout={checkout}
      />
      <div className="checkout-field-row">
        <CheckoutInput autoComplete="shipping address-level2" field="city" label="CITY" checkout={checkout} />
        <CheckoutInput autoComplete="shipping country-name" field="country" label="COUNTRY" checkout={checkout} />
      </div>
      <CheckoutInput
        autoComplete="tel"
        field="phone"
        inputMode="tel"
        label="PHONE"
        placeholder="123 456 7890"
        required={false}
        type="tel"
        checkout={checkout}
      />
      <PaymentDetails />
      <BillingAddress />
      <CheckoutErrorMessage message={checkout.checkoutError} />
      <CheckoutSubmitButton disabled={isCartEmpty || checkout.isExpressSubmitting} isSubmitting={checkout.isSubmitting} />
    </form>
  );
}

function PaymentDetails() {
  return (
    <section className="checkout-payment" aria-label="Payment details">
      <h2>PAYMENT DETAILS</h2>
      <label>
        CARD NUMBER
        <input
          autoComplete="cc-number"
          inputMode="numeric"
          name="demoCardNumber"
          placeholder="4242 4242 4242 4242"
          type="text"
        />
      </label>
      <div className="checkout-field-row">
        <label>
          EXPIRATION
          <input
            autoComplete="cc-exp"
            inputMode="numeric"
            name="demoCardExpiry"
            placeholder="04 / 28"
            type="text"
          />
        </label>
        <label>
          SECURITY CODE
          <input
            autoComplete="cc-csc"
            inputMode="numeric"
            name="demoCardSecurityCode"
            placeholder="123"
            type="text"
          />
        </label>
      </div>
    </section>
  );
}

function BillingAddress() {
  return (
    <section className="checkout-billing" aria-label="Billing address">
      <h2>BILLING ADDRESS</h2>
      <label className="checkout-checkbox">
        <input defaultChecked name="sameAsShipping" type="checkbox" />
        <span>SAME AS SHIPPING ADDRESS</span>
      </label>
    </section>
  );
}

function ExpressCheckoutButtons({
  disabled,
  isSubmitting,
  onClick,
}: {
  disabled: boolean;
  isSubmitting: boolean;
  onClick: () => void;
}) {
  const label = isSubmitting ? "STARTING..." : "CHECKOUT";
  return (
    <section className="express-checkout" aria-label="Express checkout">
      <h2>EXPRESS CHECKOUT</h2>
      <button
        className="express-checkout-button primary-pay"
        disabled={disabled || isSubmitting}
        onClick={onClick}
        type="button"
      >
        {label}
      </button>
    </section>
  );
}

function CheckoutDivider() {
  return <span className="checkout-divider">OR CONTINUE BELOW</span>;
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
  inputMode,
  label,
  placeholder = "",
  required = true,
  type = "text",
}: {
  autoComplete: string;
  checkout: CheckoutState;
  field: keyof CheckoutFields;
  inputMode?: "email" | "numeric" | "search" | "tel" | "text" | "url";
  label: string;
  placeholder?: string;
  required?: boolean;
  type?: string;
}) {
  return (
    <label>
      {label}
      <input
        autoComplete={autoComplete}
        inputMode={inputMode}
        name={field}
        onChange={(event) => checkout.updateCheckoutField(field, event.currentTarget.value)}
        placeholder={placeholder}
        required={required}
        spellCheck={field === "email" ? false : undefined}
        type={type}
        value={checkout.checkoutFields[field]}
      />
    </label>
  );
}

export function useCart(products: Product[]) {
  const [cart, setCart] = useState<CartQuantities>({});
  const [cartSizes, setCartSizes] = useState<Record<string, string>>({});
  const cartLines = useMemo(() => buildCartLines(products, cart, cartSizes), [cart, cartSizes, products]);
  const subtotalCents = useMemo(() => getSubtotalCents(cartLines), [cartLines]);
  const activeShippingCents =
    cartLines.length === 0 || subtotalCents >= freeShippingThresholdCents ? 0 : shippingCents;
  const totalCents = subtotalCents + activeShippingCents;
  const totalItems = cartLines.reduce((total, line) => total + line.quantity, 0);

  return {
    activeShippingCents,
    addToCart: (productId: string, selectedSize?: string) => {
      updateCartQuantity(setCart, productId, 1);
      if (selectedSize !== undefined) {
        setCartSizes((currentSizes) => ({ ...currentSizes, [productId]: selectedSize }));
      }
    },
    cartLines,
    clearCart: () => {
      setCart({});
      setCartSizes({});
    },
    decreaseQuantity: (productId: string) => updateCartQuantity(setCart, productId, -1),
    removeFromCart: (productId: string) => {
      removeCartLine(setCart, productId);
      removeCartSize(setCartSizes, productId);
    },
    subtotalCents,
    totalCents,
    totalItems,
  };
}

export function useCheckout(cartLines: CartLine[], totalCents: number, clearCart: () => void) {
  const [checkoutFields, setCheckoutFields] = useState<CheckoutFields>(emptyCheckoutFields);
  const [checkoutError, setCheckoutError] = useState("");
  const [isExpressSubmitting, setIsExpressSubmitting] = useState(false);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [orderReceipt, setOrderReceipt] = useState<OrderReceipt | null>(null);

  async function submitOrder(event: FormEvent<HTMLFormElement>) {
    event.preventDefault();
    setCheckoutError("");
    setIsSubmitting(true);
    try {
      const receipt = await reserveOrder(checkoutFields, cartLines, totalCents);
      setOrderReceipt(receipt);
      setCheckoutFields(emptyCheckoutFields());
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
      setOrderReceipt({ id: result.orderId, totalCents });
      setCheckoutFields(emptyCheckoutFields());
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

function emptyCheckoutFields(): CheckoutFields {
  return {
    apartment: "",
    address: "",
    city: "",
    country: "",
    email: "",
    firstName: "",
    lastName: "",
    phone: "",
  };
}

function buildCartLines(products: Product[], cart: CartQuantities, cartSizes: Record<string, string>) {
  return products
    .map((product) => {
      const quantity = cart[product.id] ?? 0;
      const selectedSize = cartSizes[product.id];
      return selectedSize === undefined ? { product, quantity } : { product, quantity, selectedSize };
    })
    .filter((line) => line.quantity > 0);
}

function removeCartSize(
  setCartSizes: (update: (currentSizes: Record<string, string>) => Record<string, string>) => void,
  productId: string,
) {
  setCartSizes((currentSizes) => {
    const nextSizes = { ...currentSizes };
    delete nextSizes[productId];
    return nextSizes;
  });
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
