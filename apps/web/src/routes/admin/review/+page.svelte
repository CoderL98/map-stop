<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { api, getToken, getUser } from '$lib/api';
	import { mountMapHost, type MapHost, type MapKind } from '$lib/map-host';
	import type { UploadItem } from '$lib/types';
	import type { Circle, Marker } from 'leaflet';

	type StatusTab = 'pending' | 'approved' | 'rejected' | 'all';

	let items = $state<UploadItem[]>([]);
	let tab = $state<StatusTab>('pending');
	let error = $state('');
	let msg = $state('');
	let rejectReasons = $state<Record<string, string>>({});
	let selected = $state<Record<string, boolean>>({});
	let activeId = $state<string | null>(null);

	let mapEl: HTMLDivElement;
	let host: MapHost | null = null;
	let mapCancel: (() => void) | null = null;
	let mapKind = $state<MapKind>('leaflet');
	let crsLabel = $state('CRS 未知');

	let previewCircle: Circle | null = null;
	let previewMarker: Marker | null = null;
	let amapPreviewOverlays: any[] = [];

	const pendingIds = $derived(items.filter((i) => i.status === 'pending').map((i) => i.id));
	const selectedPending = $derived(pendingIds.filter((id) => selected[id]));

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
	}

	function clearPreview() {
		if (mapKind === 'amap' && host?.amap) {
			if (amapPreviewOverlays.length) {
				host.amap.remove(amapPreviewOverlays);
				amapPreviewOverlays = [];
			}
			return;
		}
		if (previewCircle) {
			previewCircle.remove();
			previewCircle = null;
		}
		if (previewMarker) {
			previewMarker.remove();
			previewMarker = null;
		}
	}

	function showOnMap(it: UploadItem) {
		activeId = it.id;
		clearPreview();

		if (mapKind === 'amap' && host?.amap && host.AMap) {
			const AMap = host.AMap;
			const marker = new AMap.Marker({
				position: [it.lon, it.lat],
				title: it.name,
				map: host.amap
			});
			const circle = new AMap.Circle({
				center: [it.lon, it.lat],
				radius: it.radius_m,
				strokeColor: '#c62828',
				fillColor: '#ef5350',
				fillOpacity: 0.2,
				strokeWeight: 2
			});
			circle.setMap(host.amap);
			amapPreviewOverlays = [marker, circle];
			host.amap.setZoomAndCenter(15, [it.lon, it.lat]);
			return;
		}

		const L = host?.L;
		const map = host?.map;
		if (!L || !map) return;
		previewMarker = L.marker([it.lat, it.lon]).addTo(map).bindTooltip(it.name);
		previewCircle = L.circle([it.lat, it.lon], {
			radius: it.radius_m,
			color: '#c62828',
			fillOpacity: 0.2
		}).addTo(map);
		map.setView([it.lat, it.lon], 15);
	}

	async function load() {
		try {
			const q = tab === 'all' ? 'all' : tab;
			items = await api<UploadItem[]>(`/admin/uploads?status=${q}`);
			selected = {};
			if (items.length && tab === 'pending') {
				showOnMap(items[0]);
			}
		} catch (err) {
			error = err instanceof Error ? err.message : '加载失败';
		}
	}

	async function setTab(t: StatusTab) {
		tab = t;
		error = '';
		msg = '';
		await load();
	}

	async function approve(id: string) {
		error = '';
		msg = '';
		try {
			await api(`/admin/uploads/${id}/approve`, { method: 'POST', json: {} });
			msg = '已通过并写入系统点';
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '失败';
		}
	}

	async function reject(id: string) {
		const reason = (rejectReasons[id] || '').trim();
		if (!reason) {
			error = '请填写驳回原因';
			return;
		}
		error = '';
		msg = '';
		try {
			await api(`/admin/uploads/${id}/reject`, { method: 'POST', json: { reason } });
			msg = '已驳回';
			rejectReasons[id] = '';
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '失败';
		}
	}

	async function batchApprove() {
		const ids = selectedPending;
		if (!ids.length) {
			error = '请先勾选待审项';
			return;
		}
		error = '';
		msg = '';
		let ok = 0;
		const fails: string[] = [];
		for (const id of ids) {
			try {
				await api(`/admin/uploads/${id}/approve`, { method: 'POST', json: {} });
				ok += 1;
			} catch (err) {
				fails.push(err instanceof Error ? err.message : id);
			}
		}
		msg = `批量通过 ${ok} 条` + (fails.length ? `，失败 ${fails.length} 条` : '');
		if (fails.length) error = fails.join('；');
		await load();
	}

	function statusLabel(s: string) {
		return { pending: '待审', approved: '已通过', rejected: '已驳回' }[s] || s;
	}

	function toggleAllPending(checked: boolean) {
		const next = { ...selected };
		for (const id of pendingIds) next[id] = checked;
		selected = next;
	}

	function fmtTime(s?: string | null) {
		if (!s) return '-';
		return s.replace('T', ' ').slice(0, 19);
	}
</script>

<div class="stack">
	<div class="card">
		<h1 style="margin-top:0;">上传审核</h1>
		<div class="tabs">
			<button type="button" class:active={tab === 'pending'} onclick={() => setTab('pending')}>待审</button>
			<button type="button" class:active={tab === 'approved'} onclick={() => setTab('approved')}>已通过</button>
			<button type="button" class:active={tab === 'rejected'} onclick={() => setTab('rejected')}>已驳回</button>
			<button type="button" class:active={tab === 'all'} onclick={() => setTab('all')}>全部</button>
		</div>
		{#if error}<p class="err">{error}</p>{/if}
		{#if msg}<p class="ok">{msg}</p>{/if}

		{#if tab === 'pending' && pendingIds.length}
			<div class="row" style="margin-bottom:0.75rem;">
				<label class="row" style="gap:0.35rem;">
					<input
						type="checkbox"
						checked={selectedPending.length === pendingIds.length && pendingIds.length > 0}
						onchange={(e) => toggleAllPending((e.currentTarget as HTMLInputElement).checked)}
					/>
					全选待审
				</label>
				<button type="button" onclick={batchApprove} disabled={!selectedPending.length}>
					批量通过 ({selectedPending.length})
				</button>
			</div>
		{/if}

		{#if items.length === 0}
			<p class="muted">暂无记录</p>
		{:else}
			<table>
				<thead>
					<tr>
						{#if tab === 'pending' || tab === 'all'}
							<th></th>
						{/if}
						<th>状态</th>
						<th>用户</th>
						<th>名称</th>
						<th>坐标</th>
						<th>半径</th>
						<th>备注</th>
						<th>提交时间</th>
						<th>审核/原因</th>
						<th></th>
					</tr>
				</thead>
				<tbody>
					{#each items as it}
						<tr class:active-row={activeId === it.id}>
							{#if tab === 'pending' || tab === 'all'}
								<td>
									{#if it.status === 'pending'}
										<input type="checkbox" bind:checked={selected[it.id]} />
									{/if}
								</td>
							{/if}
							<td><span class="badge {it.status}">{statusLabel(it.status)}</span></td>
							<td>{it.username || '-'}</td>
							<td>
								<button type="button" class="secondary" style="padding:0.2rem 0.5rem;" onclick={() => showOnMap(it)}>
									{it.name}
								</button>
							</td>
							<td>{it.lat.toFixed(5)}, {it.lon.toFixed(5)}</td>
							<td>{it.radius_m}</td>
							<td>{it.note || '-'}</td>
							<td class="muted">{fmtTime(it.created_at)}</td>
							<td>
								{#if it.status === 'rejected'}
									<span class="err">{it.reject_reason || '-'}</span>
									<div class="muted">{fmtTime(it.reviewed_at)}</div>
								{:else if it.status === 'approved'}
									<span class="ok">已写入系统点</span>
									<div class="muted">{fmtTime(it.reviewed_at)}</div>
								{:else}
									<input
										placeholder="驳回原因"
										bind:value={rejectReasons[it.id]}
										style="min-width:120px;"
									/>
								{/if}
							</td>
							<td class="row">
								{#if it.status === 'pending'}
									<button type="button" onclick={() => approve(it.id)}>通过</button>
									<button type="button" class="danger" onclick={() => reject(it.id)}>驳回</button>
								{:else}
									<button type="button" class="secondary" onclick={() => showOnMap(it)}>定位</button>
								{/if}
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>

	<div class="card">
		<h3 style="margin-top:0;">选中项地图</h3>
		<p class="muted">
			点击名称可在地图上预览点与半径。
			{crsLabel} · 底图 {mapKind === 'amap' ? '高德 JS' : 'Leaflet/OSM'}
			{#if mapKind === 'amap'}（GCJ-02）{/if}
		</p>
		<div class="map map-sm" bind:this={mapEl}></div>
	</div>
</div>

<style>
	:global(tr.active-row) {
		background: #f0f9ff;
	}
</style>
