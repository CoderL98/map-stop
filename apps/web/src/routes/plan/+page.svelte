<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { api, getToken } from '$lib/api';
	import type { CustomPoint, PlanResult, SystemPoint } from '$lib/types';
	import type { Map as LMap, LayerGroup, Polyline, CircleMarker, Circle } from 'leaflet';

	let mapEl: HTMLDivElement;
	let map: LMap | null = null;
	let routeLayer: Polyline | null = null;
	let markersLayer: LayerGroup | null = null;
	let avoidLayer: LayerGroup | null = null;

	let mode = $state<'driving' | 'walking' | 'cycling'>('driving');
	let start = $state<{ lat: number; lon: number } | null>(null);
	let end = $state<{ lat: number; lon: number } | null>(null);
	let pick: 'start' | 'end' | null = $state('start');
	let systemPoints = $state<SystemPoint[]>([]);
	let customPoints = $state<CustomPoint[]>([]);
	let result = $state<PlanResult | null>(null);
	let error = $state('');
	let info = $state('');
	let loading = $state(false);
	let boundsNote = $state('杭州西湖演示路网');

	onMount(async () => {
		if (!getToken()) {
			window.location.href = '/login';
			return;
		}
		const L = (await import('leaflet')).default;
		await import('leaflet/dist/leaflet.css');
		// Fix default marker icons in Vite
		// @ts-expect-error leaflet icon hack
		delete L.Icon.Default.prototype._getIconUrl;
		L.Icon.Default.mergeOptions({
			iconRetinaUrl: 'https://unpkg.com/leaflet@1.9.4/dist/images/marker-icon-2x.png',
			iconUrl: 'https://unpkg.com/leaflet@1.9.4/dist/images/marker-icon.png',
			shadowUrl: 'https://unpkg.com/leaflet@1.9.4/dist/images/marker-shadow.png'
		});

		map = L.map(mapEl).setView([30.26, 120.15], 13);
		L.tileLayer('https://{s}.tile.openstreetmap.org/{z}/{x}/{y}.png', {
			maxZoom: 19,
			attribution: '&copy; <a href="https://www.openstreetmap.org/copyright">OpenStreetMap</a>'
		}).addTo(map);
		markersLayer = L.layerGroup().addTo(map);
		avoidLayer = L.layerGroup().addTo(map);

		map.on('click', (e) => {
			if (!pick) return;
			const { lat, lng } = e.latlng;
			if (pick === 'start') start = { lat, lon: lng };
			else end = { lat, lon: lng };
			redrawMarkers(L);
			pick = pick === 'start' ? 'end' : null;
		});

		try {
			const meta = await api<{ note: string; region: string }>('/meta/demo-bounds');
			boundsNote = `${meta.region} — ${meta.note}`;
		} catch {
			/* ignore */
		}

		await refreshPoints();
		redrawAvoids(L);
	});

	onDestroy(() => {
		map?.remove();
	});

	async function refreshPoints() {
		systemPoints = await api<SystemPoint[]>('/system-points');
		customPoints = await api<CustomPoint[]>('/custom-points');
	}

	function redrawMarkers(L: typeof import('leaflet')) {
		if (!markersLayer || !map) return;
		markersLayer.clearLayers();
		if (start) {
			L.circleMarker([start.lat, start.lon], {
				radius: 8,
				color: '#1565c0',
				fillColor: '#42a5f5',
				fillOpacity: 1
			})
				.bindTooltip('起点')
				.addTo(markersLayer);
		}
		if (end) {
			L.circleMarker([end.lat, end.lon], {
				radius: 8,
				color: '#c62828',
				fillColor: '#ef5350',
				fillOpacity: 1
			})
				.bindTooltip('终点')
				.addTo(markersLayer);
		}
	}

	function redrawAvoids(L: typeof import('leaflet')) {
		if (!avoidLayer || !map) return;
		avoidLayer.clearLayers();
		for (const p of systemPoints.filter((x) => x.enabled)) {
			L.circle([p.lat, p.lon], {
				radius: p.radius_m,
				color: '#c62828',
				fillColor: '#ef5350',
				fillOpacity: 0.25,
				weight: 1
			})
				.bindTooltip(`系统: ${p.name}`)
				.addTo(avoidLayer);
		}
		for (const p of customPoints.filter((x) => x.selected)) {
			L.circle([p.lat, p.lon], {
				radius: p.radius_m,
				color: '#f57c00',
				fillColor: '#ffb74d',
				fillOpacity: 0.25,
				weight: 1
			})
				.bindTooltip(`自定义: ${p.name}`)
				.addTo(avoidLayer);
		}
	}

	async function plan() {
		error = '';
		info = '';
		result = null;
		if (!start || !end) {
			error = '请先在地图上设置起点与终点';
			return;
		}
		loading = true;
		try {
			const L = (await import('leaflet')).default;
			const res = await api<PlanResult>('/plan', {
				method: 'POST',
				json: {
					start,
					end,
					mode,
					custom_point_ids: customPoints.filter((c) => c.selected).map((c) => c.id)
				}
			});
			result = res;
			if (routeLayer) {
				routeLayer.remove();
				routeLayer = null;
			}
			routeLayer = L.polyline(res.properties.polyline, { color: '#0b6bcb', weight: 5 }).addTo(map!);
			map!.fitBounds(routeLayer.getBounds(), { padding: [30, 30] });
			info = `距离 ${(res.properties.distance_m / 1000).toFixed(2)} km，约 ${Math.round(res.properties.duration_s / 60)} 分钟，生效躲避点 ${res.properties.avoid_count} 个`;
		} catch (err) {
			error = err instanceof Error ? err.message : '规划失败';
			if (routeLayer) {
				routeLayer.remove();
				routeLayer = null;
			}
		} finally {
			loading = false;
		}
	}

	async function toggleCustom(p: CustomPoint) {
		const L = (await import('leaflet')).default;
		const updated = await api<CustomPoint>(`/custom-points/${p.id}`, {
			method: 'PUT',
			json: {
				name: p.name,
				lat: p.lat,
				lon: p.lon,
				radius_m: p.radius_m,
				note: p.note,
				selected: !p.selected
			}
		});
		customPoints = customPoints.map((c) => (c.id === p.id ? updated : c));
		redrawAvoids(L);
	}
</script>

<div class="stack">
	<div class="card">
		<h1 style="margin:0 0 0.5rem;">路线规划</h1>
		<p style="margin:0;color:#64748b;font-size:0.9rem;">{boundsNote}</p>
		<div class="row" style="margin-top:0.75rem;">
			<button type="button" class={pick === 'start' ? '' : 'secondary'} onclick={() => (pick = 'start')}>
				设起点{start ? ` (${start.lat.toFixed(4)}, ${start.lon.toFixed(4)})` : ''}
			</button>
			<button type="button" class={pick === 'end' ? '' : 'secondary'} onclick={() => (pick = 'end')}>
				设终点{end ? ` (${end.lat.toFixed(4)}, ${end.lon.toFixed(4)})` : ''}
			</button>
			<select bind:value={mode} style="width:auto;">
				<option value="driving">驾车</option>
				<option value="walking">步行</option>
				<option value="cycling">骑行</option>
			</select>
			<button type="button" onclick={plan} disabled={loading}>{loading ? '规划中…' : '规划路线'}</button>
		</div>
		{#if error}<p class="err">{error}</p>{/if}
		{#if info}<p class="ok">{info}</p>{/if}
	</div>

	<div class="map" bind:this={mapEl}></div>

	<div class="card">
		<h3 style="margin-top:0;">自定义躲避点（勾选参与本次规划）</h3>
		{#if customPoints.length === 0}
			<p style="color:#64748b;">暂无。可在「自定义点」页添加。</p>
		{:else}
			<table>
				<thead>
					<tr><th>生效</th><th>名称</th><th>半径(m)</th></tr>
				</thead>
				<tbody>
					{#each customPoints as p}
						<tr>
							<td><input type="checkbox" checked={p.selected} onchange={() => toggleCustom(p)} /></td>
							<td>{p.name}</td>
							<td>{p.radius_m}</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
		<p style="font-size:0.85rem;color:#64748b;">红色圆：已启用系统点；橙色圆：已勾选自定义点。服务端硬避开并二次校验折线。</p>
	</div>
</div>
