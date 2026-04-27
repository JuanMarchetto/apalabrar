import { render, screen } from "@solidjs/testing-library";
import { describe, expect, it } from "vitest";
import { App } from "./App";

describe("App", () => {
  it("renders the product name", () => {
    render(() => <App />);
    expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Apalabrar");
  });
});
