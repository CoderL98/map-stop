<script lang="ts">
	import { onMount } from 'svelte';
	import { api, getToken } from '$lib/api';
	import type { UploadItem } from '$lib/types';

	let items = $state<UploadItem[]>([]);
	let name = $state('');
	let lat = $state(30.25);
	let lon = $state(120.14);
	let radius_m = $state(80);
	let note = $state('');
	let error = $state('');
	let msg = $state('');
	let fileInput: HTMLInputElement;

	onMount(async () => {
		if (!getToken()) {
			window.location.href = '/login';
			return;
		}
		await load();
	});

	async function load() {
		items = await api<UploadItem[]>('/uploads');
	}

	async function add(e: Event) {
		e.preventDefault();
		error = '';
		msg = '';
		try {
			await api('/uploads', {
				method: 'POST',
				json: { name, lat, lon, radius_m, note: note || null }
			});
			msg = '已提交，等待管理员审核';
			name = '';
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '失败';
		}
	}

	async function uploadCsv() {
		error = '';
		msg = '';
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
		const data = await res.json();
		if (!res.ok) {
			error = data.error || '导入失败';
			return;
		}
		msg = `已提交 ${data.imported} 条候选` + (data.errors?.length ? `，错误 ${data.errors.length} 条` : '');
		await load();
	}

	function statusLabel(s: string) {
		return { pending: '待审', approved: '通过', rejected: '驳回' }[s] || s;
	}
</script>

<div class="stack">
	<div class="card">
		<h1 style="margin-top:0;">上传候选躲避点</h1>
		<p style="color:#64748b;font-size:0.9rem;">通过后进入系统库并对所有用户规划生效。CSV：名称,纬度,经度,半径米,备注</p>
		<form class="stack" onsubmit={add}>
			<div class="row">
				<div style="flex:1;min-width:140px;"><label>名称</label><input bind:value={name} required /></div>
				<div style="width:120px;"><label>纬度</label><input type="number" step="any" bind:value={lat} required /></div>
				<div style="width:120px;"><label>经度</label><input type="number" step="any" bind:value={lon} required /></div>
				<div style="width:100px;"><label>半径米</label><input type="number" bind:value={radius_m} required /></div>
			</div>
			<div><label>备注</label><input bind:value={note} /></div>
			<button type="submit">提交审核</button>
		</form>
		<div class="row" style="margin-top:1rem;">
			<input type="file" accept=".csv,text/csv" bind:this={fileInput} />
			<button type="button" class="secondary" onclick={uploadCsv}>CSV 批量上传</button>
		</div>
		{#if error}<p class="err">{error}</p>{/if}
		{#if msg}<p class="ok">{msg}</p>{/if}
	</div>

	<div class="card">
		<h3 style="margin-top:0;">我的上传记录</h3>
		<table>
			<thead>
				<tr><th>状态</th><th>名称</th><th>坐标</th><th>半径</th><th>原因</th></tr>
			</thead>
			<tbody>
				{#each items as it}
					<tr>
						<td><span class="badge {it.status}">{statusLabel(it.status)}</span></td>
						<td>{it.name}</td>
						<td>{it.lat.toFixed(5)}, {it.lon.toFixed(5)}</td>
						<td>{it.radius_m}</td>
						<td>{it.reject_reason || '-'}</td>
					</tr>
				{/each}
			</tbody>
		</table>
	</div>
</div>
