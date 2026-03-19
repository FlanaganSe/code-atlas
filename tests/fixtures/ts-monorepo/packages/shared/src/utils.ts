import type { Config } from "./types";

export function greet(name: string): string {
  return `Hello, ${name}!`;
}

export function configure(config: Config): void {
  console.log(config);
}
