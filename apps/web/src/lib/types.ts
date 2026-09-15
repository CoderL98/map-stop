export type SystemPoint = {
	id: string;
	name: string;
	lat: number;
	lon: number;
	radius_m: number;
	note?: string | null;
	enabled: boolean;
	created_at: string;
	updated_at: string;
};

export type CustomPoint = {
	id: string;
	name: string;
	lat: number;
	lon: number;
	radius_m: number;
	note?: string | null;
	selected: boolean;
	created_at: string;
	updated_at: string;
};

export type UploadItem = {
	id: string;
	name: string;
	lat: number;
	lon: number;
	radius_m: number;
	note?: string | null;
	status: string;
	reject_reason?: string | null;
	created_at: string;
	reviewed_at?: string | null;
	username?: string | null;
};

export type ImportRowError = {
	line: number;
	message: string;
};

export type ImportResult = {
	imported: number;
	errors: ImportRowError[];
};

export type PlanResult = {
	type: string;
	geometry: { type: string; coordinates: [number, number][] };
	properties: {
		distance_m: number;
		duration_s: number;
		mode: string;
		avoid_count: number;
		polyline: [number, number][];
		provider?: string;
		crs?: string;
	};
};

/** Active routing / map meta from GET /api/meta/routing */
export type RoutingMeta = {
	provider: string;
	requested_provider?: string;
	engine: string;
	crs: string;
	hard_avoid: boolean;
	fallback_warning?: string | null;
	amap_configured?: boolean;
	amap_js_key?: string | null;
	amap_security_js_code?: string | null;
	geocoder?: string;
	region: string;
	bounds: {
		lat_min: number;
		lon_min: number;
		lat_max: number;
		lon_max: number;
	};
	center: { lat: number; lon: number };
	limitations?: string[];
	note: string;
	switch_path?: string;
};

/** @deprecated alias — same payload as RoutingMeta */
export type DemoBounds = RoutingMeta;

export type GeocodeHit = {
	display_name: string;
	lat: number;
	lon: number;
	name?: string | null;
	typ?: string | null;
};
