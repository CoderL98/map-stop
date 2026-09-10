<script lang="ts">
	import { api, setSession } from '$lib/api';

	let username = $state('');
	let email = $state('');
	let password = $state('');
	let error = $state('');
	let loading = $state(false);

	async function submit(e: Event) {
		e.preventDefault();
		error = '';
		loading = true;
		try {
			const res = await api<{ token: string; user: { id: string; username: string; email?: string; role: string } }>(
				'/auth/register',
				{
					method: 'POST',
					json: { username, email: email || null, password }
				}
			);
			setSession(res.token, res.user);
			window.location.href = '/plan';
		} catch (err) {
			error = err instanceof Error ? err.message : '注册失败';
		} finally {
			loading = false;
		}
	}
</script>

<div class="card" style="max-width:420px;margin:2rem auto;">
	<h1>注册</h1>
	<form class="stack" onsubmit={submit}>
		<div>
			<label for="username">用户名</label>
			<input id="username" bind:value={username} required minlength="3" />
		</div>
		<div>
			<label for="email">邮箱（可选）</label>
			<input id="email" type="email" bind:value={email} />
		</div>
		<div>
			<label for="password">密码（至少 6 位）</label>
			<input id="password" type="password" bind:value={password} required minlength="6" />
		</div>
		{#if error}<p class="err">{error}</p>{/if}
		<button type="submit" disabled={loading}>{loading ? '提交中…' : '注册'}</button>
	</form>
	<p style="margin-top:1rem;">已有账号？<a href="/login">登录</a></p>
</div>
