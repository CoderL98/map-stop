/** Shared map bootstrap: prefer 高德 JS when key present, else Leaflet/OSM. */

import { api } from '$lib/api';
import { loadAmap } from '$lib/amap';
import type { RoutingMeta } from '$lib/types';
import type { Map as LMap } from 'leaflet';

export type MapKind = 'amap' | 'leaflet';

export type MapHost = {
	kind: MapKind;
	meta: RoutingMeta | null;
	crsLabel: string;
	L: typeof import('leaflet') | null;
	map: LMap | null;
	AMap: any;
	amap: any;
	destroy: () => void;
};

/** Same policy as plan page: JS key + gaode provider or amap_configured. */
export function shouldUseAmap(meta: RoutingMeta | null | undefined): boolean {
	return (
		!!meta?.amap_js_key && (meta.provider === 'gaode' || !!meta.amap_configured)
	);
}

export function formatCrsLabel(
	meta: RoutingMeta | null | undefined,
	kind: MapKind
): string {
	if (kind === 'amap') {
		return 'CRS GCJ-02 · 高德';
	}
	if (meta?.crs) {
		return `CRS ${meta.crs} · ${meta.provider ?? 'leaflet'}`;
	}
	return 'CRS 未知 · Leaflet/OSM';
}

export async function fetchRoutingMeta(): Promise<RoutingMeta | null> {
	try {
		return await api<RoutingMeta>('/meta/routing');
	} catch {
		try {
			return await api<RoutingMeta>('/meta/demo-bounds');
		} catch {
			return null;
		}
	}
}

export async function prepareLeaflet(): Promise<typeof import('leaflet')> {
	const mod = await import('leaflet');
	const L = mod.default as unknown as typeof import('leaflet');
	await import('leaflet/dist/leaflet.css');
	// @ts-expect-error leaflet default icon paths under bundlers
	delete L.Icon.Default.prototype._getIconUrl;
	L.Icon.Default.mergeOptions({
		iconRetinaUrl: 'https://unpkg.com/leaflet@1.9.4/dist/images/marker-icon-2x.png',
		iconUrl: 'https://unpkg.com/leaflet@1.9.4/dist/images/marker-icon.png',
		shadowUrl: 'https://unpkg.com/leaflet@1.9.4/dist/images/marker-shadow.png'
	});
	return L;
}

export type CreateMapHostOpts = {
	zoom?: number;
	center?: { lat: number; lon: number };
};

/**
 * Create a map on `el`: AMap when `/meta/routing` exposes `amap_js_key`,
 * otherwise Leaflet + OSM. Always returns a usable host (AMap failure → Leaflet).
 */
export async function createMapHost(
	el: HTMLDivElement,
	opts: CreateMapHostOpts = {}
): Promise<MapHost> {
	const meta = await fetchRoutingMeta();
	const center = opts.center ?? meta?.center ?? { lat: 30.26, lon: 120.15 };
	const zoom = opts.zoom ?? 13;

	if (shouldUseAmap(meta) && meta?.amap_js_key) {
		try {
			const AMap = await loadAmap({
				jsKey: meta.amap_js_key,
				securityJsCode: meta.amap_security_js_code
			});
			const amap = new AMap.Map(el, {
				zoom,
				center: [center.lon, center.lat],
				viewMode: '2D'
			});
			const kind: MapKind = 'amap';
			return {
				kind,
				meta,
				crsLabel: formatCrsLabel(meta, kind),
				L: null,
				map: null,
				AMap,
				amap,
				destroy: () => {
					try {
						amap?.destroy?.();
					} catch {
						/* ignore */
					}
				}
			};
		} catch (e) {
			console.warn('AMap load failed, falling back to Leaflet', e);
		}
	}

	const L = await prepareLeaflet();
	const map = L.map(el).setView([center.lat, center.lon], zoom);
	const kind: MapKind = 'leaflet';
	const crsLabel = formatCrsLabel(meta, kind);
	L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
		maxZoom: 19,
		attribution:
			'&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a> · ' +
			crsLabel
	}).addTo(map);

	return {
		kind,
		meta,
		crsLabel,
		L,
		map,
		AMap: null,
		amap: null,
		destroy: () => {
			map.remove();
		}
	};
}
