<script lang="ts">
	let CheckCircle = 'check-circle';
	let AlertCircle = 'alert-circle';
	let Loader2 = 'loader';

	interface Props {
		uploading: boolean;
		uploadProgress: number;
		uploadStatus: string;
		error: string;
		showProgress?: boolean;
	}

	let { uploading, uploadProgress, uploadStatus, error, showProgress = true }: Props = $props();

	const shouldShowProgress = $derived(uploading && uploadProgress > 0 && showProgress);
</script>

<!-- Progress bar -->
{#if shouldShowProgress}
	<div class="progress-container">
		<div class="progress-bar">
			<div class="progress-fill" style="width: {uploadProgress}%"></div>
		</div>
		<div class="progress-info">
			<span class="progress-text">{uploadProgress.toFixed(0)}%</span>
			{#if uploading}
				<div class="progress-icon">⟳</div>
			{/if}
		</div>
	</div>
{/if}

<!-- Status messages -->
{#if uploadStatus}
	<div class="status success">
		<div class="status-icon">✓</div>
		<span>{uploadStatus}</span>
	</div>
{/if}

{#if error}
	<div class="status error">
		<div class="status-icon">⚠</div>
		<span>{error}</span>
	</div>
{/if}

<style>
	:global(body) {
		margin: 0;
		padding: 0;
		min-height: 100vh;
		background: linear-gradient(135deg, #232f3e 0%, #131a22 100%);
		font-family: 'Amazon Ember', 'Helvetica Neue', Roboto, Arial, sans-serif;
	}

	:global(*) {
		box-sizing: border-box;
	}

	.progress-container {
		display: flex;
		align-items: center;
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.progress-bar {
		flex: 1;
		height: 8px;
		background: #d5dbdb;
		border-radius: 4px;
		overflow: hidden;
		border: 1px solid #aab7b8;
	}

	.progress-fill {
		height: 100%;
		background: linear-gradient(90deg, #ff9900 0%, #ffb84d 50%, #ff9900 100%);
		transition: width 0.3s ease;
		border-radius: 2px;
		position: relative;
	}

	.progress-fill::after {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		bottom: 0;
		right: 0;
		background: linear-gradient(
			90deg,
			transparent 30%,
			rgba(255, 255, 255, 0.3) 50%,
			transparent 70%
		);
		animation: shimmer 2s infinite;
	}

	@keyframes shimmer {
		0% {
			transform: translateX(-100%);
		}
		100% {
			transform: translateX(100%);
		}
	}

	.progress-text {
		font-weight: 700;
		min-width: 50px;
		color: #ff9900;
		font-size: 1rem;
	}

	.status {
		padding: 1rem 1.25rem;
		border-radius: 4px;
		margin-bottom: 1.5rem;
		font-weight: 500;
		font-size: 0.95rem;
		border: 1px solid;
		position: relative;
		overflow: hidden;
	}

	.status::before {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 4px;
		opacity: 1;
	}

	.status.success {
		background: #d4edda;
		color: #155724;
		border-color: #c3e6cb;
	}

	.status.success::before {
		background: #28a745;
	}

	.status.error {
		background: #f8d7da;
		color: #721c24;
		border-color: #f5c6cb;
	}

	.status.error::before {
		background: #dc3545;
	}
</style>
