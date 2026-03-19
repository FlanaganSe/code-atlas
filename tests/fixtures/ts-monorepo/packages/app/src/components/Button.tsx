import { greet } from "@fixture/shared";

export function Button({ name }: { name: string }) {
  return <button onClick={() => alert(greet(name))}>Click me</button>;
}
