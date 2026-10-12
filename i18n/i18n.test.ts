import { describe, expect, it } from "vitest";

import en from "./en";
import tr from "./tr";

function shape(value: unknown): unknown {
  if (Array.isArray(value)) return value.length;
  if (value && typeof value === "object") {
    return Object.fromEntries(Object.entries(value).map(([k, v]) => [k, shape(v)]));
  }
  return typeof value;
}

describe("dictionaries", () => {
  it("have identical keys and list lengths", () => {
    expect(shape(en)).toEqual(shape(tr));
  });

  it("have no empty strings", () => {
    const strings = JSON.stringify([tr, en]).match(/""/g);
    expect(strings).toBeNull();
  });
});
