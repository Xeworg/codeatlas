// Reusable markdown renderer for AI responses
// Supports: headings, bold, italic, code, lists, blockquotes, links.

interface MarkdownViewProps {
  content: string
  className?: string
}

function escapeHtml(text: string): string {
  return text
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;')
    .replace(/"/g, '&quot;')
    .replace(/'/g, '&#039;')
}

function renderInlineMarkdown(text: string): string {
  // Bold + italic + inline code + links
  return text
    .replace(/\*\*(.+?)\*\*/g, '<strong>$1</strong>')
    .replace(/\*(.+?)\*/g, '<em>$1</em>')
    .replace(/`([^`]+)`/g, '<code class="px-1 py-0.5 bg-gray-100 rounded text-sm font-mono text-purple-700">$1</code>')
    .replace(/\[([^\]]+)\]\(([^)]+)\)/g, '<a href="$2" target="_blank" rel="noopener noreferrer" class="text-blue-600 underline hover:text-blue-800">$1</a>')
}

export function MarkdownView({ content, className = '' }: MarkdownViewProps) {
  const lines = content.split('\n')
  const elements: string[] = []
  let inCodeBlock = false
  let codeLines: string[] = []
  let listItems: string[] = []
  let blockquoteLines: string[] = []

  const flushList = () => {
    if (listItems.length > 0) {
      elements.push(`<ul class="list-disc list-inside space-y-1 my-2">${listItems.join('')}</ul>`)
      listItems = []
    }
  }

  const flushBlockquote = () => {
    if (blockquoteLines.length > 0) {
      elements.push(`<blockquote class="border-l-4 border-blue-300 pl-4 italic text-gray-600 my-2">${blockquoteLines.join(' ')}</blockquote>`)
      blockquoteLines = []
    }
  }

  for (let i = 0; i < lines.length; i++) {
    const line = lines[i]

    // Code block start/end
    if (line.startsWith('```')) {
      if (!inCodeBlock) {
        flushList()
        flushBlockquote()
        inCodeBlock = true
        codeLines = []
      } else {
        const lang = codeLines[0]?.trim() || ''
        const code = codeLines.slice(1).join('\n')
        elements.push(
          `<pre class="bg-gray-900 text-gray-100 rounded-lg p-4 my-3 overflow-x-auto text-sm"><code class="${lang ? `language-${lang}` : ''}">${escapeHtml(code)}</code></pre>`
        )
        codeLines = []
        inCodeBlock = false
      }
      continue
    }

    if (inCodeBlock) {
      codeLines.push(line)
      continue
    }

    flushBlockquote()

    // Headings
    if (line.startsWith('### ')) {
      flushList()
      elements.push(`<h3 class="text-base font-semibold mt-4 mb-2 text-gray-800">${renderInlineMarkdown(line.slice(4))}</h3>`)
    } else if (line.startsWith('## ')) {
      flushList()
      elements.push(`<h2 class="text-lg font-bold mt-4 mb-2 text-gray-900">${renderInlineMarkdown(line.slice(3))}</h2>`)
    } else if (line.startsWith('# ')) {
      flushList()
      elements.push(`<h1 class="text-xl font-bold mt-4 mb-2 text-gray-900">${renderInlineMarkdown(line.slice(2))}</h1>`)
    }
    // Blockquote
    else if (line.startsWith('> ')) {
      flushList()
      blockquoteLines.push(renderInlineMarkdown(line.slice(2)))
    }
    // List item
    else if (/^[-*]\s/.test(line) || /^\d+\.\s/.test(line)) {
      flushBlockquote()
      const item = line.replace(/^[-*]\s/, '').replace(/^\d+\.\s/, '')
      listItems.push(`<li class="text-gray-700">${renderInlineMarkdown(item)}</li>`)
    }
    // Horizontal rule
    else if (/^---+$/.test(line) || /^\*\*\*+$/.test(line)) {
      flushList()
      elements.push('<hr class="my-4 border-gray-300" />')
    }
    // Empty line
    else if (line.trim() === '') {
      flushList()
      elements.push('<br />')
    }
    // Regular paragraph
    else {
      flushList()
      if (line.trim()) {
        elements.push(`<p class="text-gray-700 my-2 leading-relaxed">${renderInlineMarkdown(line)}</p>`)
      }
    }
  }

  flushList()
  flushBlockquote()

  return (
    <div
      className={`prose prose-sm max-w-none text-gray-700 ${className}`}
      dangerouslySetInnerHTML={{ __html: elements.join('') }}
    />
  )
}