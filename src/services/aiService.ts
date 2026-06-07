// aiService — domain-oriented wrapper for AI operations
// Part of PR-8 (Frontend services/hooks)
// Bridges the gap between components and tauri-api for AI interactions.

import {
  explainNode as _explainNode,
  chat as _chat,
  configureAI as _configureAI,
  getAIConfig as _getAIConfig,
} from '../lib/tauri-api'
import type { NodeExplanation, ChatResponse, AIConfig } from '../lib/types'

// ─── Node explanation ───────────────────────────────────────────────────────

/**
 * Request an AI explanation for a specific node in the project graph.
 */
export async function explainNode(nodeId: string, projectId: string): Promise<NodeExplanation> {
  return _explainNode(nodeId, projectId)
}

// ─── Chat ──────────────────────────────────────────────────────────────────

/**
 * Send a chat message with project context.
 */
export async function chat(
  projectId: string,
  message: string,
  history: { id: string; role: string; content: string; timestamp: string }[],
  contextNodeIds?: string[]
): Promise<ChatResponse> {
  return _chat(projectId, message, history, contextNodeIds)
}

// ─── AI configuration ──────────────────────────────────────────────────────

/**
 * Save AI provider configuration (API key, model, endpoint).
 */
export async function configureAI(config: AIConfig): Promise<void> {
  return _configureAI(config)
}

/**
 * Load the current AI provider configuration (without the API key).
 */
export async function getAIConfig(): Promise<Omit<AIConfig, 'api_key'>> {
  return _getAIConfig()
}
