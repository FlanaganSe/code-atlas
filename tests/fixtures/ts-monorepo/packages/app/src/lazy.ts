// This file exercises dynamic imports and path aliases
export async function loadModule() {
  const mod = await import("./components/Button");
  return mod.Button;
}
