import { describe, expect, it } from "vitest";
import { localeFiles, type MessageTree } from "@/i18n";

function leafKeys(tree: MessageTree, prefix = ""): string[] {
  return Object.entries(tree).flatMap(([k, v]) => {
    const path = prefix ? `${prefix}.${k}` : k;
    return typeof v === "string" ? [path] : leafKeys(v, path);
  });
}

const { cs, en } = localeFiles;

describe("locales", () => {
  it("each locale directory has the same namespace files", () => {
    expect(Object.keys(cs).length).toBeGreaterThan(0);
    expect(Object.keys(cs).sort()).toEqual(Object.keys(en).sort());
  });

  it("cs and en have identical key sets", () => {
    expect(leafKeys(cs).sort()).toEqual(leafKeys(en).sort());
  });

  it("has no empty strings", () => {
    for (const tree of [cs, en]) {
      const empty = leafKeys(tree).filter((k) =>
        k.split(".").reduce<MessageTree | string>((node, part) => (node as MessageTree)[part] as MessageTree | string, tree) === "",
      );
      expect(empty).toEqual([]);
    }
  });
});
