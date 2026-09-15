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
	username?: string | null;
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
	};
};

export type DemoBounds = {
	region: string;
	engine: string;
	hard_avoid: boolean;
	bounds: {
		lat_min: number;
		lon_min: number;
		lat_max: number;
		lon_max: number;
	};
	center: { lat: number; lon: number };
	note: string;
};

export type GeocodeHit = {
	display_name: string;
	lat: number;
	lon: number;
	name?: string | null;
	typ?: string | null;
};
