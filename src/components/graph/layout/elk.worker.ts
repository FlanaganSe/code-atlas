/**
 * ELK layout Web Worker.
 *
 * Uses the bundled ELK build which includes the full algorithm.
 * Receives ELK graph JSON via postMessage, runs elk.layout(),
 * and posts the positioned result back.
 *
 * PRD NF3/NF10: UI thread never blocked by layout.
 */

import ELK from "elkjs/lib/elk.bundled.js";

const elk = new ELK();

self.addEventListener("message", (event: MessageEvent) => {
	elk
		.layout(event.data)
		.then((result: unknown) => {
			self.postMessage({ type: "success", data: result });
		})
		.catch((error: unknown) => {
			self.postMessage({
				type: "error",
				error: error instanceof Error ? error.message : String(error),
			});
		});
});
