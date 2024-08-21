/*!teahttp :3*/

export function isNode() {
	return typeof process === "object" && typeof require === "function";
}

export function isWeb() {
	return typeof window === "object";
}

export function isWorker() {
	return typeof importScripts === "function";
}

export function isShell() {
	return !isWeb() && !isNode() && !isWorker();
}
