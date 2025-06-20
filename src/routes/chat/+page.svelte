<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { tick } from 'svelte';
	import { generateResponseFromMessage } from './queryData';
	import { page } from '$app/stores';
	import type { LayoutData } from './$types';

	import { pollStatus } from './queryData';
	import BuzzEgg from '../../lib/Egg/buzzEgg.svelte';
	import ChatHeader from '../../lib/ChatHeader/header.svelte';
	import ChatMessage from '../../lib/ChatMessage/message.svelte';

	import TypingIndicator from '../../lib/TypingIndicator/indicator.svelte';
	import ChatInput from '../../lib/ChatInput/input.svelte';

	// Type definitions
	interface Message {
		id: number;
		type: 'user' | 'assistant';
		content: string;
		timestamp: Date;
	}

	interface ApiResponse {
		statusCode: number;
		response_message: string;
	}

	let { data }: { data: LayoutData } = $props();
	let job_id: string | null = $derived($page.url.searchParams.get('id'));

	let messages: Message[] = $state([
		{
			id: 1,
			type: 'assistant',
			content: 'Hello! Ask Buzz anything about your data that you uploaded!',
			timestamp: new Date()
		}
	]);

	let currentMessage: string = $state('');
	let isTyping: boolean = $state(false);
	let chatContainer: HTMLDivElement;
	let showEasterEgg: boolean = $state(false);

	// Polling state
	let isParquetReady: boolean = $state(false);
	let isPolling: boolean = $state(false);
	let pollingInterval: number | null = null;
	let key: string | null = null;

	// Auto-scroll to bottom when new messages are added
	$effect(() => {
		if (chatContainer && messages.length > 0) {
			tick().then(() => {
				chatContainer.scrollTop = chatContainer.scrollHeight;
			});
		}
	});

	async function startPolling(): Promise<void> {
		if (!job_id || !data.env.CORE_API_URL) {
			console.error('Missing job_id or CORE_API_URL');
			return;
		}

		isPolling = true;

		const poll = async (): Promise<void> => {
			try {
				const result = await pollStatus(data.env.CORE_API_URL!, job_id!);

				if (result.parquet_complete) {
					isParquetReady = true;
					isPolling = false;
					if (pollingInterval) {
						clearInterval(pollingInterval);
						pollingInterval = null;
					}

					messages = [
						{
							id: 1,
							type: 'assistant',
							content: 'Hello! Your data is ready! Ask Buzz anything about your uploaded data!',
							timestamp: new Date()
						}
					];
				}
			} catch (error) {
				console.error('Polling error:', error);
				if (messages.length === 1 && messages[0].content.includes('processing')) {
					messages = [
						{
							id: 1,
							type: 'assistant',
							content: 'There was an issue processing your data. Please try uploading again.',
							timestamp: new Date()
						}
					];
				}
				isPolling = false;
				if (pollingInterval) {
					clearInterval(pollingInterval);
					pollingInterval = null;
				}
			}
		};

		await poll();

		if (!isParquetReady && isPolling) {
			pollingInterval = setInterval(poll, 5000);
		}
	}

	function triggerBuzzLightyearEasterEgg(): void {
		showEasterEgg = true;

		const buzzResponse: Message = {
			id: Date.now() + 2,
			type: 'assistant',
			content:
				"🚀 TO INFINITY AND BEYOND! 🚀\n\nYou found the secret! I may be a data assistant, but I've got the heart of a Space Ranger! Now, what can this space-age AI help you discover in your data? The universe of insights awaits!",
			timestamp: new Date()
		};

		messages = [...messages, buzzResponse];

		setTimeout(() => {
			showEasterEgg = false;
		}, 3000);
	}

	async function sendMessage(): Promise<void> {
		if (!currentMessage.trim() || isTyping) return;

		if (!isParquetReady && job_id) {
			const waitingMessage: Message = {
				id: Date.now(),
				type: 'assistant',
				content:
					"Please wait while your data is being processed. I'll let you know when it's ready!",
				timestamp: new Date()
			};
			messages = [...messages, waitingMessage];
			return;
		}

		// Check for easter egg trigger
		const message = currentMessage.trim().toLowerCase();
		const isEasterEggTrigger =
			message.includes('to infinity and beyond') ||
			message.includes('infinity and beyond') ||
			message === 'to infinity and beyond!' ||
			message === 'infinity and beyond!';

		const userMessage: Message = {
			id: Date.now(),
			type: 'user',
			content: currentMessage.trim(),
			timestamp: new Date()
		};

		messages = [...messages, userMessage];
		const messageToSend: string = currentMessage;
		currentMessage = '';

		if (isEasterEggTrigger) {
			triggerBuzzLightyearEasterEgg();
			return;
		}

		isTyping = true;

		try {
			const responseContent: string = await generateResponse(messageToSend);

			const aiResponse: Message = {
				id: Date.now() + 1,
				type: 'assistant',
				content: responseContent,
				timestamp: new Date()
			};

			messages = [...messages, aiResponse];
		} catch (error) {
			console.error('Error generating response:', error);

			const errorResponse: Message = {
				id: Date.now() + 1,
				type: 'assistant',
				content: 'Sorry, I encountered an error while processing your request. Please try again.',
				timestamp: new Date()
			};

			messages = [...messages, errorResponse];
		} finally {
			isTyping = false;
		}
	}

	async function generateResponse(userMessage: string): Promise<string> {
		const responses: ApiResponse = await generateResponseFromMessage(
			data.env.CORE_API_URL!,
			userMessage,
			`parquet/${job_id}.parquet`,
			job_id
		);

		return responses.response_message;
	}

	function handleKeydown(event: KeyboardEvent): void {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			sendMessage();
		}
	}

	function clearChat(): void {
		const initialMessage = isParquetReady
			? "Hello! I'm Buzz. How can I help you today?"
			: "Hello! Your data is being processed. I'll let you know when it's ready!";

		messages = [
			{
				id: 1,
				type: 'assistant',
				content: initialMessage,
				timestamp: new Date()
			}
		];
	}

	function handleEasterEggComplete(): void {
		showEasterEgg = false;
	}

	function handleMessageChange(value: string): void {
		currentMessage = value;
	}

	onMount(() => {
		if (job_id) {
			messages = [
				{
					id: 1,
					type: 'assistant',
					content:
						"Hello! Your data is being processed. I'll let you know when it's ready to query!",
					timestamp: new Date()
				}
			];

			startPolling();
		} else {
			isParquetReady = true;
		}
	});

	onDestroy(() => {
		if (pollingInterval) {
			clearInterval(pollingInterval);
			pollingInterval = null;
		}
	});
</script>

<div class="chat-container">
	<BuzzEgg show={showEasterEgg} duration={3000} onComplete={handleEasterEggComplete} />

	<ChatHeader {isPolling} onClearChat={clearChat} />

	<main class="chat-main">
		<div class="messages-container" bind:this={chatContainer}>
			{#each messages as message (message.id)}
				<ChatMessage {message} />
			{/each}

			<TypingIndicator show={isTyping} />
		</div>
	</main>

	<ChatInput
		value={currentMessage}
		isDisabled={isTyping || !isParquetReady}
		{isParquetReady}
		onSend={sendMessage}
		onKeydown={handleKeydown}
		onValueChange={handleMessageChange}
	/>
</div>

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

	.chat-container {
		display: flex;
		flex-direction: column;
		height: 100vh;
		max-width: 1200px;
		margin: 0 auto;
		background: #ffffff;
		border-radius: 0;
		box-shadow: 0 0 20px rgba(0, 0, 0, 0.1);
		position: relative;
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

	.chat-main {
		flex: 1;
		overflow: hidden;
		background: #fafafa;
	}

	.messages-container {
		height: 100%;
		overflow-y: auto;
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
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
		.chat-container {
			height: 100vh;
			border-radius: 0;
		}
	}

	/* Scrollbar styling */
	.messages-container::-webkit-scrollbar {
		width: 6px;
	}

	.messages-container::-webkit-scrollbar-track {
		background: #f1f1f1;
		border-radius: 3px;
	}

	.messages-container::-webkit-scrollbar-thumb {
		background: #aab7b8;
		border-radius: 3px;
	}

	.messages-container::-webkit-scrollbar-thumb:hover {
		background: #8a9ba8;
	}
</style>
