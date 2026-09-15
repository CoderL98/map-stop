<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { api, getToken, getUser } from '$lib/api';
	import { mountMapHost, type MapHost, type MapKind } from '$lib/map-host';
	import type { ImportResult, SystemPoint } from '$lib/types';
	import type { LayerGroup, Circle, Marker } from 'leaflet';

	let points = $state<SystemPoint[]>([]);
	let name = $state('');
	let lat = $state(30.27);
	let lon = $state(120.16);
	let radius_m = $state(80);
	let note = $state('');
	let error = $state('');
	let msg = $state('');
	let importErrors = $state<{ line: number; message: string }[]>([]);
	let fileInput: HTMLInputElement;

	let search = $state('');
	let enabledFilter = $state<'all' | 'on' | 'off'>('all');

	let editing = $state<SystemPoint | null>(null);
	let editName = $state('');
	let editLat = $state(0);
	let editLon = $state(0);
	let editRadius = $state(80);
	let editNote = $state('');
	let editEnabled = $state(true);

	let mapEl: HTMLDivElement;
	let host: MapHost | null = null;
	let mapCancel: (() => void) | null = null;
	let mapKind = $state<MapKind>('leaflet');
	let crsLabel = $state('CRS 未知');

	// Leaflet
	let avoidLayer: LayerGroup | null = null;
	let pickMarker: Marker | null = null;
	let circleById = new Map<string, Circle>();

	// AMap
	let amapAvoidOverlays: any[] = [];
	let amapPickMarker: any = null;
	let amapInfo: any = null;
	let amapCircleById = new Map<string, any>();

	const filtered = $derived(
		points.filter((p) => {
			const q = search.trim().toLowerCase();
			if (q && !p.name.toLowerCase().includes(q) && !(p.note || '').toLowerCase().includes(q)) {
				return false;
			}
			if (enabledFilter === 'on' && !p.enabled) return false;
			if (enabledFilter === 'off' && p.enabled) return false;
			return true;
		})
	);

	const enabledCount = $derived(points.filter((p) => p.enabled).length);
	const disabledCount = $derived(points.length - enabledCount);

	onMount(async () => {
		if (!getToken() || getUser()?.role !== 'admin') {
			window.location.href = '/login';
			return;
		}
		await initMap();
		await load();
	});

	onDestroy(() => {
		mapCancel?.();
		mapCancel = null;
		host = null;
	});

	async function initMap() {
		const mounted = mountMapHost(mapEl);
		mapCancel = mounted.cancel;
		host = await mounted.ready;
		if (!host) return;
		mapKind = host.kind;
		crsLabel = host.crsLabel;

		if (host.kind === 'amap' && host.amap) {
			host.amap.on('click', (e: any) => {
				lat = Number(e.lnglat.getLat().toFixed(6));
				lon = Number(e.lnglat.getLng().toFixed(6));
				redrawPick();
			});
		} else if (host.map && host.L) {
			avoidLayer = host.L.layerGroup().addTo(host.map);
			host.map.on('click', (e) => {
				lat = Number(e.latlng.lat.toFixed(6));
				lon = Number(e.latlng.lng.toFixed(6));
				redrawPick();
			});
		}
	}

	function redrawPick() {
		if (mapKind === 'amap' && host?.amap && host.AMap) {
			const AMap = host.AMap;
			if (amapPickMarker) {
				amapPickMarker.setPosition([lon, lat]);
			} else {
				amapPickMarker = new AMap.Marker({
					position: [lon, lat],
					draggable: true,
					title: '新建点位置',
					map: host.amap
				});
				amapPickMarker.on('dragend', () => {
					const p = amapPickMarker.getPosition();
					lat = Number(p.getLat().toFixed(6));
					lon = Number(p.getLng().toFixed(6));
				});
			}
			return;
		}
		const L = host?.L;
		const map = host?.map;
		if (!L || !map) return;
		if (pickMarker) {
			pickMarker.setLatLng([lat, lon]);
		} else {
			pickMarker = L.marker([lat, lon], { draggable: true, title: '新建点位置' }).addTo(map);
			pickMarker.on('dragend', () => {
				const ll = pickMarker!.getLatLng();
				lat = Number(ll.lat.toFixed(6));
				lon = Number(ll.lng.toFixed(6));
			});
		}
	}

	function redrawAvoids() {
		if (mapKind === 'amap' && host?.amap && host.AMap) {
			redrawAmapAvoids();
			return;
		}
		const L = host?.L;
		const map = host?.map;
		if (!L || !avoidLayer || !map) return;
		avoidLayer.clearLayers();
		circleById.clear();
		const bounds: [number, number][] = [];
		for (const p of points) {
			const color = p.enabled ? '#c62828' : '#94a3b8';
			const c = L.circle([p.lat, p.lon], {
				radius: p.radius_m,
				color,
				weight: 2,
				fillColor: color,
				fillOpacity: p.enabled ? 0.18 : 0.08
			})
				.bindTooltip(`${p.name} (${p.radius_m}m)`)
				.addTo(avoidLayer);
			circleById.set(p.id, c);
			bounds.push([p.lat, p.lon]);
		}
		if (bounds.length > 0) {
			map.fitBounds(bounds as [number, number][], { padding: [30, 30], maxZoom: 15 });
		}
		redrawPick();
	}

	function redrawAmapAvoids() {
		const AMap = host!.AMap;
		const amap = host!.amap;
		if (amapAvoidOverlays.length) {
			amap.remove(amapAvoidOverlays);
			amapAvoidOverlays = [];
		}
		amapCircleById.clear();
		for (const p of points) {
			const color = p.enabled ? '#c62828' : '#94a3b8';
			const c = new AMap.Circle({
				center: [p.lon, p.lat],
				radius: p.radius_m,
				strokeColor: color,
				strokeWeight: 2,
				fillColor: color,
				fillOpacity: p.enabled ? 0.18 : 0.08
			});
			c.setMap(amap);
			c.setExtData({ id: p.id, name: p.name, radius_m: p.radius_m });
			amapAvoidOverlays.push(c);
			amapCircleById.set(p.id, c);
		}
		if (amapAvoidOverlays.length) {
			amap.setFitView(amapAvoidOverlays, false, [30, 30, 30, 30], 15);
		}
		redrawPick();
	}

	async function load() {
		try {
			points = await api<SystemPoint[]>('/system-points');
			redrawAvoids();
		} catch (err) {
			error = err instanceof Error ? err.message : '加载失败';
		}
	}

	async function add(e: Event) {
		e.preventDefault();
		error = '';
		msg = '';
		importErrors = [];
		try {
			await api('/system-points', {
				method: 'POST',
				json: { name, lat, lon, radius_m, note: note || null, enabled: true }
			});
			msg = '已创建';
			name = '';
			note = '';
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '失败';
		}
	}

	function openEdit(p: SystemPoint) {
		editing = p;
		editName = p.name;
		editLat = p.lat;
		editLon = p.lon;
		editRadius = p.radius_m;
		editNote = p.note || '';
		editEnabled = p.enabled;
		focusOn(p);
	}

	async function saveEdit(e: Event) {
		e.preventDefault();
		if (!editing) return;
		error = '';
		msg = '';
		try {
			await api(`/system-points/${editing.id}`, {
				method: 'PATCH',
				json: {
					name: editName,
					lat: editLat,
					lon: editLon,
					radius_m: editRadius,
					note: editNote || null,
					enabled: editEnabled
				}
			});
			msg = '已保存';
			editing = null;
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '失败';
		}
	}

	async function toggle(p: SystemPoint) {
		error = '';
		try {
			await api(`/system-points/${p.id}/enabled`, {
				method: 'PATCH',
				json: { enabled: !p.enabled }
			});
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '更新失败';
		}
	}

	async function remove(id: string) {
		if (!confirm('确认删除该系统点？')) return;
		error = '';
		try {
			await api(`/system-points/${id}`, { method: 'DELETE' });
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '删除失败';
		}
	}

	async function importCsv() {
		error = '';
		msg = '';
		importErrors = [];
		const file = fileInput?.files?.[0];
		if (!file) {
			error = '请选择 CSV';
			return;
		}
		const fd = new FormData();
		fd.append('file', file);
		const token = getToken();
		const res = await fetch('/api/system-points/import', {
			method: 'POST',
			headers: token ? { Authorization: `Bearer ${token}` } : {},
			body: fd
		});
		const data = (await res.json()) as ImportResult & { error?: string };
		if (!res.ok) {
			error = data.error || '导入失败';
			return;
		}
		importErrors = Array.isArray(data.errors) ? data.errors : [];
		msg = `导入 ${data.imported} 条` + (importErrors.length ? `，错误 ${importErrors.length} 条` : '');
		if (fileInput) fileInput.value = '';
		await load();
	}

	function focusOn(p: SystemPoint) {
		if (mapKind === 'amap' && host?.amap && host.AMap) {
			host.amap.setZoomAndCenter(15, [p.lon, p.lat]);
			if (amapInfo) {
				amapInfo.close();
			}
			amapInfo = new host.AMap.InfoWindow({
				content: `<strong>${p.name}</strong>（${p.radius_m}m）`,
				offset: new host.AMap.Pixel(0, -20)
			});
			amapInfo.open(host.amap, [p.lon, p.lat]);
			return;
		}
		const map = host?.map;
		if (!map) return;
		map.panTo([p.lat, p.lon]);
		circleById.get(p.id)?.openTooltip();
	}
</script>

<div class="stack">
	<div class="card">
		<h1 style="margin-top:0;">系统躲避点管理</h1>
		<p class="counts">
			共 {points.length} 个 · 启用 {enabledCount} · 停用 {disabledCount}
		</p>
		<form class="stack" onsubmit={add}>
			<div class="row">
				<div style="flex:1;min-width:140px;"><label>名称</label><input bind:value={name} required /></div>
				<div style="width:120px;"><label>纬度</label><input type="number" step="any" bind:value={lat} required /></div>
				<div style="width:120px;"><label>经度</label><input type="number" step="any" bind:value={lon} required /></div>
				<div style="width:100px;"><label>半径米</label><input type="number" bind:value={radius_m} required min="1" /></div>
			</div>
			<div><label>备注</label><input bind:value={note} /></div>
			<p class="muted">点击下方地图可填充新建点的经纬度；可拖动标记微调。</p>
			<button type="submit">新增系统点</button>
		</form>
		<div class="row" style="margin-top:1rem;">
			<input type="file" accept=".csv,text/csv" bind:this={fileInput} />
			<button type="button" class="secondary" onclick={importCsv}>CSV 导入</button>
			<a class="btn secondary" href="/avoid-points.csv" download="avoid-points.csv">下载 CSV 模板</a>
		</div>
		{#if error}<p class="err">{error}</p>{/if}
		{#if msg}<p class="ok">{msg}</p>{/if}
		{#if importErrors.length}
			<ul class="import-errors">
				{#each importErrors as er}
					<li>第 {er.line} 行：{er.message}</li>
				{/each}
			</ul>
		{/if}
	</div>

	<div class="card">
		<h3 style="margin-top:0;">地图预览</h3>
		<p class="muted">
			{crsLabel}
			· 底图 {mapKind === 'amap' ? '高德 JS' : 'Leaflet/OSM'}
			{#if mapKind === 'amap'}
				（请用本页点选，勿粘贴未转换的 WGS84/OSM 坐标）
			{/if}
		</p>
		<div class="map map-sm" bind:this={mapEl}></div>
	</div>

	<div class="card">
		<div class="row" style="margin-bottom:0.75rem;">
			<div style="flex:1;min-width:180px;">
				<label>搜索名称/备注</label>
				<input bind:value={search} placeholder="关键字…" />
			</div>
			<div style="width:140px;">
				<label>启用状态</label>
				<select bind:value={enabledFilter}>
					<option value="all">全部</option>
					<option value="on">仅启用</option>
					<option value="off">仅停用</option>
				</select>
			</div>
		</div>
		<table>
			<thead>
				<tr><th>启用</th><th>名称</th><th>坐标</th><th>半径</th><th>备注</th><th></th></tr>
			</thead>
			<tbody>
				{#each filtered as p}
					<tr>
						<td><input type="checkbox" checked={p.enabled} onchange={() => toggle(p)} /></td>
						<td>
							<button type="button" class="secondary" style="padding:0.2rem 0.5rem;" onclick={() => focusOn(p)}>
								{p.name}
							</button>
						</td>
						<td>{p.lat.toFixed(5)}, {p.lon.toFixed(5)}</td>
						<td>{p.radius_m} m</td>
						<td>{p.note || '-'}</td>
						<td class="row">
							<button type="button" class="secondary" onclick={() => openEdit(p)}>编辑</button>
							<button class="danger" type="button" onclick={() => remove(p.id)}>删除</button>
						</td>
					</tr>
				{/each}
			</tbody>
		</table>
		{#if filtered.length === 0}
			<p class="muted">无匹配项</p>
		{/if}
	</div>
</div>

{#if editing}
	<div class="modal-backdrop" role="presentation" onclick={(e) => { if (e.target === e.currentTarget) editing = null; }}>
		<div class="modal" role="dialog" aria-modal="true">
			<h3 style="margin-top:0;">编辑系统点</h3>
			<form class="stack" onsubmit={saveEdit}>
				<div><label>名称</label><input bind:value={editName} required /></div>
				<div class="row">
					<div style="flex:1;"><label>纬度</label><input type="number" step="any" bind:value={editLat} required /></div>
					<div style="flex:1;"><label>经度</label><input type="number" step="any" bind:value={editLon} required /></div>
					<div style="width:100px;"><label>半径米</label><input type="number" bind:value={editRadius} required min="1" /></div>
				</div>
				<div><label>备注</label><input bind:value={editNote} /></div>
				<label class="row" style="gap:0.4rem;">
					<input type="checkbox" bind:checked={editEnabled} /> 启用
				</label>
				<div class="row">
					<button type="submit">保存</button>
					<button type="button" class="secondary" onclick={() => (editing = null)}>取消</button>
				</div>
			</form>
		</div>
	</div>
{/if}
