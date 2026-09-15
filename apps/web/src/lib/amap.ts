/** Load 高德 JS API 2.0 once. */

export type AmapKeys = {
	jsKey: string;
	securityJsCode?: string | null;
};

declare global {
	interface Window {
		_AMapSecurityConfig?: { securityJsCode: string };
		AMap?: any;
	}
}

let loading: Promise<any> | null = null;

export function loadAmap(keys: AmapKeys): Promise<any> {
	if (typeof window === 'undefined') {
		return Promise.reject(new Error('no window'));
	}
	if (window.AMap) return Promise.resolve(window.AMap);
	if (loading) return loading;

	if (keys.securityJsCode) {
		window._AMapSecurityConfig = { securityJsCode: keys.securityJsCode };
	}

	loading = new Promise((resolve, reject) => {
		const script = document.createElement('script');
		script.src = `https://webapi.amap.com/maps?v=2.0&key=${encodeURIComponent(keys.jsKey)}`;
		script.async = true;
		script.onload = () => {
			if (window.AMap) resolve(window.AMap);
			else reject(new Error('AMap failed to load'));
		};
		script.onerror = () => reject(new Error('无法加载高德 JS API'));
		document.head.appendChild(script);
	}).catch((err) => {
		// Allow a later retry after a failed load (bad key / network).
		loading = null;
		throw err;
	});
	return loading;
}
