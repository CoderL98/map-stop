<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { api, getToken } from '$lib/api';
	import type { CustomPoint, DemoBounds, GeocodeHit, PlanResult, SystemPoint } from '$lib/types';
	import type { Map as LMap, LayerGroup, Polyline, Marker, Rectangle } from 'leaflet';

	let mapEl: HTMLDivElement;
	let map: LMap | null = null;
	let routeLayer: Polyline | null = null;
	let markersLayer: LayerGroup | null = null;
	let avoidLayer: LayerGroup | null = null;
	let boundsRect: Rectangle | null = null;
	let startMarker: Marker | null = null;
	let endMarker: Marker | null = null;
	let Lref: typeof import('leaflet') | null = null;

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
	let demoBounds = $state<DemoBounds | null>(null);

	let startQuery = $state('');
	let endQuery = $state('');
	let startHits = $state<GeocodeHit[]>([]);
	let endHits = $state<GeocodeHit[]>([]);
	let searchingStart = $state(false);
	let searchingEnd = $state(false);
	let searchHint = $state('');

	onMount(async () => {
		if (!getToken()) {
			window.location.href = '/login';
			return;
		}
		const L = (await import('leaflet')).default;
		Lref = L as unknown as typeof import('leaflet');
		await import('leaflet/dist/leaflet.css');
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
			if (pick === 'start') {
				start = { lat, lon: lng };
				startQuery = `${lat.toFixed(5)}, ${lng.toFixed(5)}`;
				startHits = [];
			} else {
				end = { lat, lon: lng };
				endQuery = `${lat.toFixed(5)}, ${lng.toFixed(5)}`;
				endHits = [];
			}
			redrawMarkers();
			pick = pick === 'start' ? 'end' : null;
		});

		try {
			const meta = await api<DemoBounds>('/meta/demo-bounds');
			demoBounds = meta;
			boundsNote = `${meta.region} — ${meta.note}`;
			const b = meta.bounds;
			boundsRect = L.rectangle(
				[
					[b.lat_min, b.lon_min],
					[b.lat_max, b.lon_max]
				],
				{
					color: '#1565c0',
					weight: 2,
					dashArray: '6 4',
					fillColor: '#42a5f5',
					fillOpacity: 0.06
				}
			)
				.bindTooltip('演示路网有效范围')
				.addTo(map);
			map.fitBounds(boundsRect.getBounds(), { padding: [20, 20] });
		} catch {
			/* ignore */
		}

		await refreshPoints();
		redrawAvoids();
	});

	onDestroy(() => {
		map?.remove();
	});

	async function refreshPoints() {
		systemPoints = await api<SystemPoint[]>('/system-points');
		customPoints = await api<CustomPoint[]>('/custom-points');
	}

	function blueIcon(L: typeof import('leaflet')) {
		return L.divIcon({
			className: '',
			html: `<div style="width:18px;height:18px;border-radius:50%;background:#42a5f5;border:3px solid #1565c0;box-shadow:0 1px 4px rgba(0,0,0,.35)"></div>`,
			iconSize: [18, 18],
			iconAnchor: [9, 9]
		});
	}

	function redIcon(L: typeof import('leaflet')) {
		return L.divIcon({
			className: '',
			html: `<div style="width:18px;height:18px;border-radius:50%;background:#ef5350;border:3px solid #c62828;box-shadow:0 1px 4px rgba(0,0,0,.35)"></div>`,
			iconSize: [18, 18],
			iconAnchor: [9, 9]
		});
	}

	function redrawMarkers() {
		const L = Lref;
		if (!L || !markersLayer || !map) return;
		if (startMarker) {
			startMarker.remove();
			startMarker = null;
		}
		if (endMarker) {
			endMarker.remove();
			endMarker = null;
		}
		markersLayer.clearLayers();
		if (start) {
			startMarker = L.marker([start.lat, start.lon], {
				draggable: true,
				icon: blueIcon(L),
				title: '起点（可拖动）'
			})
				.bindTooltip('起点（拖动调整）')
				.addTo(markersLayer);
			startMarker.on('dragend', () => {
				const ll = startMarker!.getLatLng();
				start = { lat: ll.lat, lon: ll.lng };
				startQuery = `${ll.lat.toFixed(5)}, ${ll.lng.toFixed(5)}`;
			});
		}
		if (end) {
			endMarker = L.marker([end.lat, end.lon], {
				draggable: true,
				icon: redIcon(L),
				title: '终点（可拖动）'
			})
				.bindTooltip('终点（拖动调整）')
				.addTo(markersLayer);
			endMarker.on('dragend', () => {
				const ll = endMarker!.getLatLng();
				end = { lat: ll.lat, lon: ll.lng };
				endQuery = `${ll.lat.toFixed(5)}, ${ll.lng.toFixed(5)}`;
			});
		}
	}

	function redrawAvoids() {
		const L = Lref;
		if (!L || !avoidLayer || !map) return;
		avoidLayer.clearLayers();
		for (const p of systemPoints.filter((x) => x.enabled)) {
			L.circle([p.lat, p.lon], {
				radius: p.radius_m,
				color: '#c62828',
				fillColor: '#ef5350',
				fillOpacity: 0.25,
				weight: 1
			})
				.bindTooltip(`系统: ${p.name}（${p.radius_m}m）`)
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
				.bindTooltip(`自定义: ${p.name}（${p.radius_m}m）`)
				.addTo(avoidLayer);
		}
	}

	async function searchPlace(which: 'start' | 'end') {
		const q = which === 'start' ? startQuery.trim() : endQuery.trim();
		if (!q) {
			searchHint = '请输入地名关键词';
			return;
		}
		searchHint = '';
		error = '';
		if (which === 'start') {
			searchingStart = true;
			startHits = [];
		} else {
			searchingEnd = true;
			endHits = [];
		}
		try {
			const hits = await api<GeocodeHit[]>(`/geocode?q=${encodeURIComponent(q)}&limit=5`);
			if (which === 'start') startHits = hits;
			else endHits = hits;
			if (hits.length === 0) searchHint = '未找到结果，可换关键词或直接在地图上点选';
		} catch (err) {
			error = err instanceof Error ? err.message : '搜索失败';
		} finally {
			if (which === 'start') searchingStart = false;
			else searchingEnd = false;
		}
	}

	function applyHit(which: 'start' | 'end', hit: GeocodeHit) {
		const pt = { lat: hit.lat, lon: hit.lon };
		if (which === 'start') {
			start = pt;
			startQuery = hit.name || hit.display_name.split(',')[0] || hit.display_name;
			startHits = [];
			if (!end) pick = 'end';
			else pick = null;
		} else {
			end = pt;
			endQuery = hit.name || hit.display_name.split(',')[0] || hit.display_name;
			endHits = [];
			pick = null;
		}
		redrawMarkers();
		map?.panTo([pt.lat, pt.lon], { animate: true });
		if (map && map.getZoom() < 14) map.setZoom(14);
	}

	function clearPoint(which: 'start' | 'end') {
		if (which === 'start') {
			start = null;
			startQuery = '';
			startHits = [];
			pick = 'start';
		} else {
			end = null;
			endQuery = '';
			endHits = [];
			pick = 'end';
		}
		redrawMarkers();
	}

	async function plan() {
		error = '';
		info = '';
		result = null;
		if (!start || !end) {
			error = '请先设置起点与终点（搜索、地图点选或拖动标记）';
			return;
		}
		loading = true;
		try {
			const L = Lref ?? ((await import('leaflet')).default as unknown as typeof import('leaflet'));
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
			// keep avoid circles + markers visible
			redrawAvoids();
			redrawMarkers();
			info = `距离 ${(res.properties.distance_m / 1000).toFixed(2)} km · 约 ${Math.round(res.properties.duration_s / 60)} 分钟 · 生效躲避点 ${res.properties.avoid_count} 个`;
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
		redrawAvoids();
	}
</script>

<div class="stack">
	<div class="card">
		<h1 style="margin:0 0 0.5rem;">路线规划</h1>
		<p style="margin:0;color:#64748b;font-size:0.9rem;">{boundsNote}</p>

		<div class="search-grid" style="margin-top:0.85rem;">
			<div class="search-col">
				<label for="start-q">起点搜索</label>
				<div class="row">
					<input
						id="start-q"
						bind:value={startQuery}
						placeholder="地名，如 断桥、西湖博物馆"
						onkeydown={(e) => e.key === 'Enter' && searchPlace('start')}
					/>
					<button type="button" class="secondary" disabled={searchingStart} onclick={() => searchPlace('start')}>
						{searchingStart ? '…' : '搜索'}
					</button>
					{#if start}
						<button type="button" class="secondary" onclick={() => clearPoint('start')}>清除</button>
					{/if}
				</div>
				{#if startHits.length > 0}
					<ul class="hits">
						{#each startHits as h}
							<li>
								<button type="button" class="hit" onclick={() => applyHit('start', h)}>
									<strong>{h.name || h.display_name.split(',')[0]}</strong>
									<span>{h.display_name}</span>
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
			<div class="search-col">
				<label for="end-q">终点搜索</label>
				<div class="row">
					<input
						id="end-q"
						bind:value={endQuery}
						placeholder="地名，如 雷峰塔、净慈寺"
						onkeydown={(e) => e.key === 'Enter' && searchPlace('end')}
					/>
					<button type="button" class="secondary" disabled={searchingEnd} onclick={() => searchPlace('end')}>
						{searchingEnd ? '…' : '搜索'}
					</button>
					{#if end}
						<button type="button" class="secondary" onclick={() => clearPoint('end')}>清除</button>
					{/if}
				</div>
				{#if endHits.length > 0}
					<ul class="hits">
						{#each endHits as h}
							<li>
								<button type="button" class="hit" onclick={() => applyHit('end', h)}>
									<strong>{h.name || h.display_name.split(',')[0]}</strong>
									<span>{h.display_name}</span>
								</button>
							</li>
						{/each}
					</ul>
				{/if}
			</div>
		</div>
		{#if searchHint}<p style="margin:0.4rem 0 0;color:#64748b;font-size:0.85rem;">{searchHint}</p>{/if}

		<div class="row" style="margin-top:0.75rem;">
			<button type="button" class={pick === 'start' ? '' : 'secondary'} onclick={() => (pick = 'start')}>
				{pick === 'start' ? '▶ 点选起点' : '重选起点'}{start ? ` (${start.lat.toFixed(4)}, ${start.lon.toFixed(4)})` : ''}
			</button>
			<button type="button" class={pick === 'end' ? '' : 'secondary'} onclick={() => (pick = 'end')}>
				{pick === 'end' ? '▶ 点选终点' : '重选终点'}{end ? ` (${end.lat.toFixed(4)}, ${end.lon.toFixed(4)})` : ''}
			</button>
			{#if pick}
				<button type="button" class="secondary" onclick={() => (pick = null)}>取消点选</button>
			{/if}
			<select bind:value={mode} style="width:auto;">
				<option value="driving">驾车</option>
				<option value="walking">步行</option>
				<option value="cycling">骑行</option>
			</select>
			<button type="button" onclick={plan} disabled={loading}>{loading ? '规划中…' : '规划路线'}</button>
		</div>
		<p style="margin:0.5rem 0 0;font-size:0.85rem;color:#64748b;">
			提示：蓝色虚线框为演示有效范围；可搜索地名、地图点击、或拖动蓝/红标记调整起终点。
		</p>
		{#if error}<p class="err">{error}</p>{/if}
		{#if result}
			<div class="stats">
				<div><span class="stats-label">距离</span><strong>{(result.properties.distance_m / 1000).toFixed(2)} km</strong></div>
				<div><span class="stats-label">预计时长</span><strong>{Math.round(result.properties.duration_s / 60)} 分钟</strong></div>
				<div><span class="stats-label">生效躲避</span><strong>{result.properties.avoid_count} 个</strong></div>
				<div><span class="stats-label">方式</span><strong>{mode === 'driving' ? '驾车' : mode === 'walking' ? '步行' : '骑行'}</strong></div>
			</div>
		{:else if info}
			<p class="ok">{info}</p>
		{/if}
	</div>

	<div class="map" bind:this={mapEl}></div>

	<div class="card">
		<h3 style="margin-top:0;">自定义躲避点（勾选参与本次规划）</h3>
		{#if customPoints.length === 0}
			<p style="color:#64748b;">
				暂无。可在
				<a href="/custom">自定义点</a>
				页添加；半径默认 80m，可在该页调整。
			</p>
		{:else}
			<table>
				<thead>
					<tr><th>生效</th><th>名称</th><th>半径(m)</th><th></th></tr>
				</thead>
				<tbody>
					{#each customPoints as p}
						<tr>
							<td><input type="checkbox" checked={p.selected} onchange={() => toggleCustom(p)} /></td>
							<td>{p.name}</td>
							<td>{p.radius_m}</td>
							<td><a href="/custom" style="font-size:0.85rem;">调整半径</a></td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
		<p style="font-size:0.85rem;color:#64748b;">
			红色圆：已启用系统点；橙色圆：已勾选自定义点。服务端硬避开并二次校验折线。
			{#if demoBounds}
				演示引擎：{demoBounds.engine}。
			{/if}
		</p>
	</div>
</div>

<style>
	.search-grid {
		display: grid;
		grid-template-columns: 1fr 1fr;
		gap: 0.75rem;
	}
	@media (max-width: 800px) {
		.search-grid {
			grid-template-columns: 1fr;
		}
	}
	.search-col .row input {
		flex: 1;
		min-width: 8rem;
	}
	.hits {
		list-style: none;
		margin: 0.35rem 0 0;
		padding: 0;
		border: 1px solid #e2e8f0;
		border-radius: 8px;
		max-height: 180px;
		overflow: auto;
		background: #fff;
	}
	.hits li {
		border-bottom: 1px solid #f1f5f9;
	}
	.hits li:last-child {
		border-bottom: none;
	}
	.hit {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.15rem;
		width: 100%;
		text-align: left;
		background: transparent;
		color: #1a1a1a;
		border-radius: 0;
		padding: 0.45rem 0.65rem;
	}
	.hit:hover {
		background: #f1f5f9;
	}
	.hit span {
		font-size: 0.78rem;
		color: #64748b;
		font-weight: 400;
	}
	.stats {
		display: flex;
		flex-wrap: wrap;
		gap: 1rem;
		margin-top: 0.75rem;
		padding: 0.75rem 1rem;
		background: #eff6ff;
		border: 1px solid #bfdbfe;
		border-radius: 10px;
	}
	.stats-label {
		display: block;
		font-size: 0.75rem;
		color: #64748b;
	}
	.stats strong {
		font-size: 1.15rem;
		color: #0b6bcb;
	}
</style>
