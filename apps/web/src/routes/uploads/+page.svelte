<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { api, getToken } from '$lib/api';
	import type { DemoBounds, ImportResult, UploadItem } from '$lib/types';
	import type { Map as LMap, Marker, Rectangle } from 'leaflet';

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
	let map: LMap | null = null;
	let pickMarker: Marker | null = null;
	let boundsRect: Rectangle | null = null;
	let Lref: typeof import('leaflet') | null = null;
	let boundsNote = $state('');

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
		map?.remove();
	});

	async function initMap() {
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
			attribution: '&copy; OpenStreetMap'
		}).addTo(map);
		map.on('click', (e) => {
			lat = Number(e.latlng.lat.toFixed(6));
			lon = Number(e.latlng.lng.toFixed(6));
			redrawPick();
		});
		try {
			const meta = await api<DemoBounds>('/meta/demo-bounds');
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
		redrawPick();
	}

	function redrawPick() {
		const L = Lref;
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
			<p class="muted">点击地图选点，或拖动标记调整坐标。</p>
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
