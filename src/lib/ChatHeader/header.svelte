<script lang="ts">
	interface Props {
		isPolling: boolean;
		onClearChat: () => void;
	}

	let { isPolling = false, onClearChat }: Props = $props();
</script>

<header class="chat-header">
	<div class="header-content">
		<div class="header-left">
			<div class="buzz-avatar">
				<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor"> </svg>
			</div>
			<div class="header-info">
				<h1>Buzz</h1>
				<p class="subtitle">
					The natural language query assistant that can help with your data and beyond
					{#if isPolling}
						<span class="processing-indicator">• Processing data...</span>
					{/if}
				</p>
			</div>
		</div>
		<!-- svelte-ignore a11y_consider_explicit_label -->
		<button class="clear-btn" onclick={onClearChat} title="Clear conversation">
			<svg
				width="20"
				height="20"
				viewBox="0 0 24 24"
				fill="none"
				stroke="currentColor"
				stroke-width="2"
			>
				<path d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6m3 0V4c0-1 1-2 2-2h4c0 1 1 2 2 2v2" />
				<line x1="10" y1="11" x2="10" y2="17" />
				<line x1="14" y1="11" x2="14" y2="17" />
			</svg>
		</button>
	</div>
</header>

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

	/* Processing indicator */
	.processing-indicator {
		color: #ff9900;
		font-weight: 500;
		animation: processingPulse 2s infinite ease-in-out;
	}

	@keyframes processingPulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.6;
		}
	}
	.chat-header {
		background: linear-gradient(90deg, #232f3e 0%, #37475a 100%);
		color: white;
		padding: 1rem 1.5rem;
		border-bottom: 1px solid #d5dbdb;
		flex-shrink: 0;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.buzz-avatar {
		width: 40px;
		height: 40px;
		background: #ff9900;
		color: #000000;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
	}

	.header-info h1 {
		margin: 0;
		font-size: 1.5rem;
		font-weight: 700;
		color: #ffffff;
		letter-spacing: -0.025em;
	}

	.header-info .subtitle {
		margin: 0;
		font-size: 0.9rem;
		color: rgba(255, 255, 255, 0.8);
		font-weight: 300;
	}

	.clear-btn {
		background: rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.2);
		color: rgba(255, 255, 255, 0.8);
		padding: 0.5rem;
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s ease;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.clear-btn:hover {
		background: rgba(255, 255, 255, 0.2);
		color: #ffffff;
		border-color: rgba(255, 255, 255, 0.3);
	}

	@keyframes slideIn {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@keyframes typingDot {
		0%,
		60%,
		100% {
			transform: scale(1);
			opacity: 0.5;
		}
		30% {
			transform: scale(1.2);
			opacity: 1;
		}
	}

	/* Responsive design */
	@media (max-width: 768px) {
		.chat-header {
			padding: 1rem;
		}

		.header-info h1 {
			font-size: 1.25rem;
		}
	}
</style>
