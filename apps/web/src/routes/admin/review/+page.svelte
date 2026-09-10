<script lang="ts">
	import { onMount } from 'svelte';
	import { api, getToken, getUser } from '$lib/api';
	import type { UploadItem } from '$lib/types';

	let items = $state<UploadItem[]>([]);
	let error = $state('');
	let msg = $state('');

	onMount(async () => {
		if (!getToken() || getUser()?.role !== 'admin') {
			window.location.href = '/login';
			return;
		}
		await load();
	});

	async function load() {
		items = await api<UploadItem[]>('/admin/uploads');
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
		const reason = prompt('请填写驳回原因');
		if (!reason) return;
		error = '';
		msg = '';
		try {
			await api(`/admin/uploads/${id}/reject`, { method: 'POST', json: { reason } });
			msg = '已驳回';
			await load();
		} catch (err) {
			error = err instanceof Error ? err.message : '失败';
		}
	}
</script>

<div class="stack">
	<div class="card">
		<h1 style="margin-top:0;">上传审核</h1>
		{#if error}<p class="err">{error}</p>{/if}
		{#if msg}<p class="ok">{msg}</p>{/if}
		{#if items.length === 0}
			<p style="color:#64748b;">暂无待审项</p>
		{:else}
			<table>
				<thead>
					<tr><th>用户</th><th>名称</th><th>坐标</th><th>半径</th><th>备注</th><th></th></tr>
				</thead>
				<tbody>
					{#each items as it}
						<tr>
							<td>{it.username}</td>
							<td>{it.name}</td>
							<td>{it.lat.toFixed(5)}, {it.lon.toFixed(5)}</td>
							<td>{it.radius_m}</td>
							<td>{it.note || '-'}</td>
							<td class="row">
								<button type="button" onclick={() => approve(it.id)}>通过</button>
								<button type="button" class="danger" onclick={() => reject(it.id)}>驳回</button>
							</td>
						</tr>
					{/each}
				</tbody>
			</table>
		{/if}
	</div>
</div>
