import { describe, expect, it } from "vitest";

import { fill } from "./i18n-format";

describe("fill", () => {
  it("replaces known placeholders only", () => {
    expect(fill("{done}/{total} · {x}", { done: 3, total: 10 })).toBe("3/10 · {x}");
  });
});
