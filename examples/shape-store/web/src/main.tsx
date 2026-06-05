import { createRootRoute, createRouter, RouterProvider } from "@tanstack/react-router";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles.css";
import "./checkout.css";

const rootRoute = createRootRoute({ component: App });
const router = createRouter({ routeTree: rootRoute });

declare module "@tanstack/react-router" {
  interface Register {
    router: typeof router;
  }
}

const rootElement = document.getElementById("root");
if (rootElement === null) {
  throw new Error("missing root element");
}

createRoot(rootElement).render(<RouterProvider router={router} />);
