/**
 * ELK layout Web Worker.
 *
 * Receives an ELK graph JSON via postMessage, runs elk.layout(),
 * and posts the positioned result back. This ensures the UI thread
 * is never blocked by layout computation (PRD NF3/NF10).
 */

import ELK from "elkjs/lib/elk.bundled.js";

const elk = new ELK();

self.onmessage = async (event: MessageEvent) => {
	try {
		const result = await elk.layout(event.data);
		self.postMessage({ type: "success", data: result });
	} catch (error) {
		self.postMessage({
			type: "error",
			error: error instanceof Error ? error.message : String(error),
		});
	}
};
