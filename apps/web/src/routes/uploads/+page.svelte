<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { api, getToken } from '$lib/api';
	import { createMapHost, type MapHost, type MapKind } from '$lib/map-host';
	import type { ImportResult, UploadItem } from '$lib/types';
	import type { Marker, Rectangle } from 'leaflet';

	let items = $state<UploadItem[]>([]);
	let name = $state('');
	let lat = $state(30.25);
	let lon = $state(120.14);
	let radius_m = $state(80);
	let note = $state('');
	let error = $state('');
	let msg = $state('');
	let importErrors = $state<{ line: number; message: string }[]>([]);
	let fileInput: HTMLInputElement;
	let statusFilter = $state<'all' | 'pending' | 'approved' | 'rejected'>('all');

	let mapEl: HTMLDivElement;
	let host: MapHost | null = null;
	let mapKind = $state<MapKind>('leaflet');
	let crsLabel = $state('CRS 未知');
	let boundsNote = $state('');

	let pickMarker: Marker | null = null;
	let boundsRect: Rectangle | null = null;
	let amapPickMarker: any = null;
	let amapBoundsRect: any = null;

	const filtered = $derived(
		statusFilter === 'all' ? items : items.filter((i) => i.status === statusFilter)
	);

	onMount(async () => {
		if (!getToken()) {
			window.location.href = '/login';
			return;
		}
		await initMap();
		await load();
	});

	onDestroy(() => {
		host?.destroy();
	});

	async function initMap() {
		host = await createMapHost(mapEl);
		mapKind = host.kind;
		crsLabel = host.crsLabel;

		if (host.meta) {
			boundsNote = `${host.meta.region} — ${host.meta.note}`;
		}

		if (host.kind === 'amap' && host.amap) {
			host.amap.on('click', (e: any) => {
				lat = Number(e.lnglat.getLat().toFixed(6));
				lon = Number(e.lnglat.getLng().toFixed(6));
				redrawPick();
			});
			maybeDrawDemoBoundsAmap();
		} else if (host.map && host.L) {
			host.map.on('click', (e) => {
				lat = Number(e.latlng.lat.toFixed(6));
				lon = Number(e.latlng.lng.toFixed(6));
				redrawPick();
			});
			maybeDrawDemoBoundsLeaflet();
		}
		redrawPick();
	}

	function maybeDrawDemoBoundsLeaflet() {
		const meta = host?.meta;
		const L = host?.L;
		const map = host?.map;
		if (!meta?.bounds || !L || !map) return;
		if (meta.provider !== 'embedded') return;
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
	}

	function maybeDrawDemoBoundsAmap() {
		const meta = host?.meta;
		const AMap = host?.AMap;
		const amap = host?.amap;
		if (!meta?.bounds || !AMap || !amap) return;
		if (meta.provider !== 'embedded') return;
		const b = meta.bounds;
		amapBoundsRect = new AMap.Rectangle({
			bounds: new AMap.Bounds([b.lon_min, b.lat_min], [b.lon_max, b.lat_max]),
			strokeColor: '#1565c0',
			strokeWeight: 2,
			strokeStyle: 'dashed',
			fillColor: '#42a5f5',
			fillOpacity: 0.06
		});
		amapBoundsRect.setMap(amap);
		amap.setFitView([amapBoundsRect]);
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
					title: '候选点',
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
			pickMarker = L.marker([lat, lon], { draggable: true, title: '候选点' }).addTo(map);
			pickMarker.on('dragend', () => {
				const ll = pickMarker!.getLatLng();
				lat = Number(ll.lat.toFixed(6));
				lon = Number(ll.lng.toFixed(6));
			});
		}
	}

	async function load() {
		items = await api<UploadItem[]>('/uploads');
	}

	async function add(e: Event) {
		e.preventDefault();
		error = '';
		msg = '';
		importErrors = [];
		try {
			await api('/uploads', {
				method: 'POST',
				json: { name, lat, lon, radius_m, note: note || null }
			});
			msg = '已提交，等待管理员审核';
			name = '';
			note = '';
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '失败';
		}
	}

	async function uploadCsv() {
		error = '';
		msg = '';
		importErrors = [];
		const file = fileInput?.files?.[0];
		if (!file) {
			error = '请选择 CSV 文件';
			return;
		}
		const fd = new FormData();
		fd.append('file', file);
		const token = getToken();
		const res = await fetch('/api/uploads/import', {
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
		msg =
			`已提交 ${data.imported} 条候选` +
			(importErrors.length ? `，错误 ${importErrors.length} 条` : '');
		if (fileInput) fileInput.value = '';
		await load();
	}

	function statusLabel(s: string) {
		return { pending: '待审', approved: '通过', rejected: '驳回' }[s] || s;
	}

	function fmtTime(s?: string | null) {
		if (!s) return '-';
		return s.replace('T', ' ').slice(0, 19);
	}

	$effect(() => {
		lat;
		lon;
		redrawPick();
	});
</script>

<div class="stack">
	<div class="card">
		<h1 style="margin-top:0;">上传候选躲避点</h1>
		<p class="muted">
			通过后进入系统库并对所有用户规划生效。CSV 列：名称,纬度,经度,半径米,备注
			{#if boundsNote}<br />{boundsNote}{/if}
		</p>
		<form class="stack" onsubmit={add}>
			<div class="row">
				<div style="flex:1;min-width:140px;"><label>名称</label><input bind:value={name} required /></div>
				<div style="width:120px;"><label>纬度</label><input type="number" step="any" bind:value={lat} required /></div>
				<div style="width:120px;"><label>经度</label><input type="number" step="any" bind:value={lon} required /></div>
				<div style="width:100px;"><label>半径米</label><input type="number" bind:value={radius_m} required min="1" /></div>
			</div>
			<div><label>备注</label><input bind:value={note} /></div>
			<p class="muted">
				点击地图选点，或拖动标记调整坐标。
				{crsLabel} · 底图 {mapKind === 'amap' ? '高德 JS' : 'Leaflet/OSM'}
				{#if mapKind === 'amap'}
					（GCJ-02：请用本页点选，勿粘贴未转换的 WGS84/OSM 坐标）
				{/if}
			</p>
			<div class="map map-sm" bind:this={mapEl}></div>
			<button type="submit">提交审核</button>
		</form>
		<div class="row" style="margin-top:1rem;">
			<input type="file" accept=".csv,text/csv" bind:this={fileInput} />
			<button type="button" class="secondary" onclick={uploadCsv}>CSV 批量上传</button>
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
		<div class="row" style="justify-content:space-between;margin-bottom:0.5rem;">
			<h3 style="margin:0;">我的上传记录</h3>
			<div style="width:140px;">
				<label>状态筛选</label>
				<select bind:value={statusFilter}>
					<option value="all">全部</option>
					<option value="pending">待审</option>
					<option value="approved">通过</option>
					<option value="rejected">驳回</option>
				</select>
			</div>
		</div>
		<table>
			<thead>
				<tr><th>状态</th><th>名称</th><th>坐标</th><th>半径</th><th>提交时间</th><th>原因</th></tr>
			</thead>
			<tbody>
				{#each filtered as it}
					<tr>
						<td><span class="badge {it.status}">{statusLabel(it.status)}</span></td>
						<td>{it.name}</td>
						<td>{it.lat.toFixed(5)}, {it.lon.toFixed(5)}</td>
						<td>{it.radius_m}</td>
						<td class="muted">{fmtTime(it.created_at)}</td>
						<td>{it.reject_reason || '-'}</td>
					</tr>
				{/each}
			</tbody>
		</table>
		{#if filtered.length === 0}
			<p class="muted">暂无记录</p>
		{/if}
	</div>
</div>
