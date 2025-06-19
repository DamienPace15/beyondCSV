<script lang="ts">
	interface Props {
		show: boolean;
	}

	let { show = false }: Props = $props();
</script>

{#if show}
	<div class="message assistant">
		<div class="message-avatar">
			<div class="avatar buzz-avatar">
				<svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"> </svg>
			</div>
		</div>
		<div class="message-content">
			<div class="typing-indicator">
				<div class="typing-dots">
					<span></span>
					<span></span>
					<span></span>
				</div>
			</div>
		</div>
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

	@keyframes processingPulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.6;
		}
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

	.message {
		display: flex;
		gap: 0.75rem;
		max-width: 800px;
		animation: slideIn 0.3s ease-out;
	}

	.message-avatar {
		flex-shrink: 0;
	}

	.avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 2px solid;
	}

	.avatar.buzz-avatar {
		background: #ff9900;
		color: #000000;
		border-color: #ff9900;
	}

	.message-content {
		flex: 1;
		min-width: 0;
	}

	.typing-indicator {
		background: #ffffff;
		padding: 1rem 1.25rem;
		border-radius: 12px;
		border: 1px solid #d5dbdb;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.typing-dots {
		display: flex;
		gap: 0.25rem;
		align-items: center;
	}

	.typing-dots span {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #aab7b8;
		animation: typingDot 1.4s infinite ease-in-out;
	}

	.typing-dots span:nth-child(2) {
		animation-delay: 0.2s;
	}

	.typing-dots span:nth-child(3) {
		animation-delay: 0.4s;
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
		.message {
			max-width: 100%;
		}
	}
</style>
