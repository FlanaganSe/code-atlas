import { greet } from "@fixture/shared";
import type { Config } from "@fixture/shared";
import { VERSION } from "@fixture/shared";
import { type LogLevel, configure } from "@fixture/shared";

const config: Config = { name: "app", version: 1 };
configure(config);

const level: LogLevel = "info";
console.log(greet("world"), VERSION, level);
