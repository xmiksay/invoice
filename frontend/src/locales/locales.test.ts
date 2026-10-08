import { describe, expect, it } from "vitest";
import cs from "./cs.json";
import en from "./en.json";

type Tree = { [k: string]: string | Tree };

function leafKeys(tree: Tree, prefix = ""): string[] {
  return Object.entries(tree).flatMap(([k, v]) => {
    const path = prefix ? `${prefix}.${k}` : k;
    return typeof v === "string" ? [path] : leafKeys(v, path);
  });
}

describe("locales", () => {
  it("cs and en have identical key sets", () => {
    expect(leafKeys(cs).sort()).toEqual(leafKeys(en).sort());
  });

  it("has no empty strings", () => {
    for (const tree of [cs, en]) {
      const empty = leafKeys(tree).filter((k) =>
        k.split(".").reduce<Tree | string>((node, part) => (node as Tree)[part], tree) === "",
      );
      expect(empty).toEqual([]);
    }
  });
});
