// This file exercises unsupported constructs
const legacy = require("./legacy");
const dynamic = import("./dynamic");

export function hello(): string {
  return "hello";
}
