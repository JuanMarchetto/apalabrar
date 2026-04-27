import { describe, expect, it } from "vitest";
import { VERSION } from "./index";

describe("@apalabrar/ui", () => {
  it("exports a pinned version", () => {
    expect(VERSION).toBe("0.0.0");
  });
});
