const TOKEN_KEY = 'mapstop_token';
const USER_KEY = 'mapstop_user';

export type User = {
	id: string;
	username: string;
	email?: string | null;
	role: string;
};

export function getToken(): string | null {
	if (typeof localStorage === 'undefined') return null;
	return localStorage.getItem(TOKEN_KEY);
}

export function getUser(): User | null {
	if (typeof localStorage === 'undefined') return null;
	const raw = localStorage.getItem(USER_KEY);
	if (!raw) return null;
	try {
		return JSON.parse(raw) as User;
	} catch {
		return null;
	}
}

export function setSession(token: string, user: User) {
	localStorage.setItem(TOKEN_KEY, token);
	localStorage.setItem(USER_KEY, JSON.stringify(user));
}

export function clearSession() {
	localStorage.removeItem(TOKEN_KEY);
	localStorage.removeItem(USER_KEY);
}

export async function api<T>(
	path: string,
	opts: RequestInit & { json?: unknown } = {}
): Promise<T> {
	const headers = new Headers(opts.headers || {});
	const token = getToken();
	if (token) headers.set('Authorization', `Bearer ${token}`);
	let body = opts.body;
	if (opts.json !== undefined) {
		headers.set('Content-Type', 'application/json');
		body = JSON.stringify(opts.json);
	}
	const { json: _j, ...rest } = opts;
	const res = await fetch(`/api${path}`, { ...rest, headers, body });
	const data = await res.json().catch(() => ({}));
	if (!res.ok) {
		throw new Error((data as { error?: string }).error || `请求失败 (${res.status})`);
	}
	return data as T;
}
