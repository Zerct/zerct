import { flushSync } from "react-dom";
import { type CSSProperties } from "react";

type ViewTransitionDocument = Document & {
  startViewTransition?: (update: () => void) => { finished: Promise<void> };
};

export function getProductTransitionStyle(productId: string): CSSProperties {
  return { viewTransitionName: `product-${productId}` };
}

export function transitionStoreState(update: () => void) {
  const transitionDocument = document as ViewTransitionDocument;
  const prefersReducedMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;

  if (prefersReducedMotion || transitionDocument.startViewTransition === undefined) {
    update();
    return;
  }

  const transition = transitionDocument.startViewTransition(() => flushSync(update));
  void transition.finished.catch(() => undefined);
}
