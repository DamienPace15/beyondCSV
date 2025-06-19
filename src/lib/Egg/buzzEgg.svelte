<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		show: boolean;
		duration?: number;
		onComplete?: () => void;
	}

	let { show = false, duration = 3000, onComplete }: Props = $props();

	onMount(() => {
		if (show && duration > 0) {
			const timeout = setTimeout(() => {
				onComplete?.();
			}, duration);

			return () => clearTimeout(timeout);
		}
	});
</script>

{#if show}
	<div class="easter-egg-overlay">
		<div class="buzz-lightyear-animation">
			<div class="rocket">🚀</div>
			<div class="stars">
				<div class="star">⭐</div>
				<div class="star">✨</div>
				<div class="star">⭐</div>
				<div class="star">✨</div>
				<div class="star">⭐</div>
			</div>
			<div class="infinity-text">TO INFINITY AND BEYOND!</div>
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

	/* Easter Egg Styles */
	.easter-egg-overlay {
		position: fixed;
		top: 0;
		left: 0;
		width: 100vw;
		height: 100vh;
		background: linear-gradient(45deg, #1a1a2e, #16213e, #0f3460);
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
		animation: easterEggFadeIn 0.5s ease-out;
	}

	.buzz-lightyear-animation {
		text-align: center;
		position: relative;
	}

	.rocket {
		font-size: 8rem;
		animation: rocketFly 2s ease-in-out;
		display: block;
		margin-bottom: 2rem;
	}

	.stars {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}

	.star {
		position: absolute;
		font-size: 2rem;
		animation: starTwinkle 1.5s infinite ease-in-out;
	}

	.star:nth-child(1) {
		top: 20%;
		left: 10%;
		animation-delay: 0s;
	}

	.star:nth-child(2) {
		top: 30%;
		right: 15%;
		animation-delay: 0.3s;
	}

	.star:nth-child(3) {
		bottom: 40%;
		left: 20%;
		animation-delay: 0.6s;
	}

	.star:nth-child(4) {
		bottom: 30%;
		right: 25%;
		animation-delay: 0.9s;
	}

	.star:nth-child(5) {
		top: 50%;
		left: 50%;
		animation-delay: 1.2s;
	}

	.infinity-text {
		font-size: 3rem;
		font-weight: bold;
		color: #00ff88;
		text-shadow:
			0 0 20px #00ff88,
			0 0 40px #00ff88;
		animation: infinityGlow 1s ease-in-out infinite alternate;
		letter-spacing: 0.1em;
	}

	@keyframes easterEggFadeIn {
		from {
			opacity: 0;
			transform: scale(0.8);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}

	@keyframes rocketFly {
		0% {
			transform: translateX(-100vw) rotate(-45deg);
		}
		50% {
			transform: translateX(0) rotate(0deg);
		}
		100% {
			transform: translateX(0) rotate(0deg) scale(1.2);
		}
	}

	@keyframes starTwinkle {
		0%,
		100% {
			opacity: 0.3;
			transform: scale(1);
		}
		50% {
			opacity: 1;
			transform: scale(1.3);
		}
	}

	@keyframes infinityGlow {
		0% {
			text-shadow:
				0 0 20px #00ff88,
				0 0 40px #00ff88;
		}
		100% {
			text-shadow:
				0 0 30px #00ff88,
				0 0 60px #00ff88,
				0 0 80px #00ff88;
		}
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
		.rocket {
			font-size: 4rem;
		}

		.infinity-text {
			font-size: 1.5rem;
		}

		.star {
			font-size: 1.5rem;
		}
	}
</style>
