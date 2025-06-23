<script lang="ts">
	import { onMount, onDestroy } from 'svelte';
	import { tick } from 'svelte';
	import { generateResponseFromMessage } from './queryData';
	import { updateContext } from './updateContext';
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

	interface PollResponse {
		statusCode: number;
		parquet_complete: boolean;
		context?: string;
		schema?: { [key: string]: string };
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

	// Schema and context state
	let schema: { [key: string]: string } = $state({});
	let context: string = $state('');
	let editableContext: string = $state('');
	let isEditingContext: boolean = $state(false);
	let sidebarVisible: boolean = $state(true);

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
				const result: PollResponse = await pollStatus(data.env.CORE_API_URL!, job_id!);

				if (result.parquet_complete) {
					isParquetReady = true;
					isPolling = false;
					if (pollingInterval) {
						clearInterval(pollingInterval);
						pollingInterval = null;
					}

					// Update schema and context from poll response
					if (result.schema) {
						schema = result.schema;
					}
					if (result.context) {
						context = result.context;
						editableContext = result.context;
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

	function toggleSidebar(): void {
		sidebarVisible = !sidebarVisible;
	}

	function startEditingContext(): void {
		isEditingContext = true;
		editableContext = context;
	}

	async function saveContext(): Promise<void> {
		try {
			await updateContext(data.env.CORE_API_URL!, editableContext, job_id!);

			context = editableContext;
			isEditingContext = false;
		} catch (error) {
			console.error('Failed to save context:', error);
		}
	}

	function cancelEditContext(): void {
		editableContext = context;
		isEditingContext = false;
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

<div class="app-container">
	<BuzzEgg show={showEasterEgg} duration={3000} onComplete={handleEasterEggComplete} />

	<div class="chat-container" class:sidebar-open={sidebarVisible}>
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

	{#if isParquetReady}
		<!-- Sidebar Toggle Button -->
		<button class="sidebar-toggle" on:click={toggleSidebar} aria-label="Toggle sidebar">
			{#if sidebarVisible}
				<svg
					width="20"
					height="20"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path d="M21 3H3c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2z" />
					<path d="M10 4v16" />
					<path d="m14 8 2 2-2 2" />
				</svg>
			{:else}
				<svg
					width="20"
					height="20"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path d="M21 3H3c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2z" />
					<path d="M10 4v16" />
					<path d="m16 12-2-2 2-2" />
				</svg>
			{/if}
		</button>

		<!-- Sidebar -->
		<div class="sidebar" class:visible={sidebarVisible}>
			<div class="sidebar-content">
				<h3>Data Schema</h3>
				<div class="schema-container">
					{#if Object.keys(schema).length > 0}
						<div class="schema-grid">
							{#each Object.entries(schema) as [field, type]}
								<div class="schema-item">
									<span class="field-name">{field}</span>
									<span class="field-type">{type}</span>
								</div>
							{/each}
						</div>
					{:else}
						<p class="no-data">No schema available</p>
					{/if}
				</div>

				<h3>Context</h3>
				<div class="context-container">
					{#if isEditingContext}
						<div class="context-edit">
							<textarea
								bind:value={editableContext}
								placeholder="Enter context about your data..."
								rows="6"
							></textarea>
							<div class="context-actions">
								<button class="save-btn" on:click={saveContext}>Save</button>
								<button class="cancel-btn" on:click={cancelEditContext}>Cancel</button>
							</div>
						</div>
					{:else}
						<div class="context-display">
							<p class="context-text">{context || 'No context available'}</p>
							<button class="edit-btn" on:click={startEditingContext}>Edit</button>
						</div>
					{/if}
				</div>
			</div>
		</div>
	{/if}
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

	.app-container {
		display: flex;
		height: 100vh;
		max-width: 100vw;
		position: relative;
	}

	.chat-container {
		display: flex;
		flex-direction: column;
		flex: 1;
		background: #ffffff;
		box-shadow: 0 0 20px rgba(0, 0, 0, 0.1);
		transition: margin-right 0.3s ease;
	}

	.chat-container.sidebar-open {
		margin-right: 350px;
	}

	.sidebar-toggle {
		position: fixed;
		top: 50%;
		right: 10px;
		transform: translateY(-50%);
		z-index: 1001;
		background: #232f3e;
		color: white;
		border: none;
		border-radius: 50%;
		width: 40px;
		height: 40px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
		transition: all 0.3s ease;
	}

	.sidebar-toggle:hover {
		background: #131a22;
		transform: translateY(-50%) scale(1.1);
	}

	.sidebar {
		position: fixed;
		top: 0;
		right: 0;
		width: 350px;
		height: 100vh;
		background: #f8f9fa;
		border-left: 1px solid #e9ecef;
		transform: translateX(100%);
		transition: transform 0.3s ease;
		z-index: 1000;
		overflow-y: auto;
	}

	.sidebar.visible {
		transform: translateX(0);
	}

	.sidebar-content {
		padding: 2rem;
	}

	.sidebar h3 {
		margin: 0 0 1rem 0;
		color: #232f3e;
		font-size: 1.2rem;
		font-weight: 600;
		border-bottom: 2px solid #ff9900;
		padding-bottom: 0.5rem;
	}

	.schema-container {
		margin-bottom: 2rem;
	}

	.schema-grid {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.schema-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		background: white;
		border: 1px solid #e9ecef;
		border-radius: 8px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.field-name {
		font-weight: 600;
		color: #232f3e;
	}

	.field-type {
		background: #ff9900;
		color: white;
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.875rem;
		font-weight: 500;
	}

	.context-container {
		background: white;
		border: 1px solid #e9ecef;
		border-radius: 8px;
		overflow: hidden;
	}

	.context-display {
		padding: 1rem;
	}

	.context-text {
		margin: 0 0 1rem 0;
		color: #495057;
		line-height: 1.5;
		min-height: 2rem;
	}

	.context-edit textarea {
		width: 100%;
		border: none;
		padding: 1rem;
		font-family: inherit;
		font-size: 0.9rem;
		resize: vertical;
		outline: none;
		background: #f8f9fa;
	}

	.context-actions {
		display: flex;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background: #f8f9fa;
		border-top: 1px solid #e9ecef;
	}

	.edit-btn,
	.save-btn,
	.cancel-btn {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.875rem;
		font-weight: 500;
		transition: all 0.2s ease;
	}

	.edit-btn {
		background: #232f3e;
		color: white;
	}

	.edit-btn:hover {
		background: #131a22;
	}

	.save-btn {
		background: #28a745;
		color: white;
	}

	.save-btn:hover {
		background: #218838;
	}

	.cancel-btn {
		background: #6c757d;
		color: white;
	}

	.cancel-btn:hover {
		background: #5a6268;
	}

	.no-data {
		color: #6c757d;
		font-style: italic;
		margin: 0;
		padding: 1rem;
		text-align: center;
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

	/* Responsive design */
	@media (max-width: 768px) {
		.chat-container.sidebar-open {
			margin-right: 0;
		}

		.sidebar {
			width: 100vw;
		}

		.sidebar-toggle {
			right: 15px;
		}
	}

	/* Scrollbar styling */
	.messages-container::-webkit-scrollbar,
	.sidebar::-webkit-scrollbar {
		width: 6px;
	}

	.messages-container::-webkit-scrollbar-track,
	.sidebar::-webkit-scrollbar-track {
		background: #f1f1f1;
		border-radius: 3px;
	}

	.messages-container::-webkit-scrollbar-thumb,
	.sidebar::-webkit-scrollbar-thumb {
		background: #aab7b8;
		border-radius: 3px;
	}

	.messages-container::-webkit-scrollbar-thumb:hover,
	.sidebar::-webkit-scrollbar-thumb:hover {
		background: #8a9ba8;
	}
</style>
