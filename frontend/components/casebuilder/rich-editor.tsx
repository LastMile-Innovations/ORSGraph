"use client"

import { useEditor, EditorContent, type Editor, type JSONContent } from "@tiptap/react"
import StarterKit from "@tiptap/starter-kit"
import Placeholder from "@tiptap/extension-placeholder"
import CharacterCount from "@tiptap/extension-character-count"
import { useEffect, useState, type ReactNode } from "react"
import { cn } from "@/lib/utils"
import { 
  Bold, 
  Italic, 
  List, 
  ListOrdered, 
  Quote, 
  Heading1, 
  Heading2, 
  Undo, 
  Redo,
  Type,
  Code,
  Sparkles,
  Search
} from "lucide-react"
import { Button } from "@/components/ui/button"
import { Separator } from "@/components/ui/separator"
import {
  Command,
  CommandEmpty,
  CommandGroup,
  CommandInput,
  CommandItem,
  CommandList,
} from "@/components/ui/command"
import { Popover, PopoverContent, PopoverTrigger } from "@/components/ui/popover"

interface RichEditorProps {
  content: string
  onChange: (content: string) => void
  onFocus?: () => void
  onSelectionChange?: (range: RichEditorTextRange | null) => void
  onSaveShortcut?: () => void
  placeholder?: string
  className?: string
  minHeight?: string
  readOnly?: boolean
}

export interface RichEditorTextRange {
  startOffset: number
  endOffset: number
  quote: string
}

const blockSeparator = "\n\n"
const hardBreakText = "\n"

export function RichEditor({ 
  content, 
  onChange, 
  onFocus, 
  onSelectionChange,
  onSaveShortcut,
  placeholder = "Start drafting...", 
  className,
  minHeight = "200px",
  readOnly = false,
}: RichEditorProps) {
  const [isSlashMenuOpen, setIsSlashMenuOpen] = useState(false)

  const editor = useEditor({
    extensions: [
      StarterKit,
      Placeholder.configure({
        placeholder,
      }),
      CharacterCount,
    ],
    content: textToTiptapDocument(content),
    editable: !readOnly,
    onUpdate: ({ editor }) => {
      onChange(editorPlainText(editor))
    },
    onSelectionUpdate: ({ editor }) => {
      onSelectionChange?.(editorSelectionRange(editor))
    },
    onFocus: () => {
      onFocus?.()
    },
    editorProps: {
      attributes: {
        class: cn(
          "prose prose-sm dark:prose-invert max-w-none focus:outline-none min-h-inherit",
          className
        ),
        style: `min-height: ${minHeight}`,
      },
      handleKeyDown: (view, event) => {
        if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
          event.preventDefault()
          onSaveShortcut?.()
          return true
        }
        if (event.key === "/") {
          if (!readOnly) setIsSlashMenuOpen(true)
          return false
        }
        return false
      },
    },
    immediatelyRender: false,
  })

  // Sync content if it changes externally
  useEffect(() => {
    if (editor && content !== editorPlainText(editor)) {
      editor.commands.setContent(textToTiptapDocument(content), { emitUpdate: false })
    }
  }, [content, editor])

  useEffect(() => {
    if (editor) {
      editor.setEditable(!readOnly)
    }
  }, [editor, readOnly])

  if (!editor) return null

  const handleCommand = (command: () => void) => {
    command()
    setIsSlashMenuOpen(false)
    editor.chain().focus().run()
  }

  return (
    <div className="flex flex-col rounded-md border border-input bg-background shadow-sm transition-all focus-within:ring-1 focus-within:ring-primary/20">
      <div className="flex flex-wrap items-center gap-1 border-b bg-muted/10 p-1">
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleBold().run()}
          active={editor.isActive("bold")}
          disabled={readOnly}
          icon={<Bold className="h-3.5 w-3.5" />}
          label="Bold"
        />
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleItalic().run()}
          active={editor.isActive("italic")}
          disabled={readOnly}
          icon={<Italic className="h-3.5 w-3.5" />}
          label="Italic"
        />
        <Separator orientation="vertical" className="mx-1 h-4" />
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleHeading({ level: 1 }).run()}
          active={editor.isActive("heading", { level: 1 })}
          disabled={readOnly}
          icon={<Heading1 className="h-3.5 w-3.5" />}
          label="H1"
        />
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleHeading({ level: 2 }).run()}
          active={editor.isActive("heading", { level: 2 })}
          disabled={readOnly}
          icon={<Heading2 className="h-3.5 w-3.5" />}
          label="H2"
        />
        <Separator orientation="vertical" className="mx-1 h-4" />
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleBulletList().run()}
          active={editor.isActive("bulletList")}
          disabled={readOnly}
          icon={<List className="h-3.5 w-3.5" />}
          label="Bullet List"
        />
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleOrderedList().run()}
          active={editor.isActive("orderedList")}
          disabled={readOnly}
          icon={<ListOrdered className="h-3.5 w-3.5" />}
          label="Numbered List"
        />
        <ToolbarButton
          onClick={() => editor.chain().focus().toggleBlockquote().run()}
          active={editor.isActive("blockquote")}
          disabled={readOnly}
          icon={<Quote className="h-3.5 w-3.5" />}
          label="Quote"
        />
        
        <div className="ml-auto flex items-center gap-1">
          <Popover open={isSlashMenuOpen} onOpenChange={setIsSlashMenuOpen}>
            <PopoverTrigger asChild>
              <Button variant="ghost" size="sm" className="h-7 gap-1.5 px-2 text-[10px] uppercase tracking-wider text-muted-foreground hover:text-primary" disabled={readOnly}>
                <Sparkles className="h-3 w-3" />
                Actions
              </Button>
            </PopoverTrigger>
            <PopoverContent className="w-56 p-0" align="end">
              <Command>
                <CommandInput placeholder="Search commands..." className="h-8 text-xs" />
                <CommandList>
                  <CommandEmpty>No results found.</CommandEmpty>
                  <CommandGroup heading="Formatting">
                    <CommandItem onSelect={() => handleCommand(() => editor.chain().focus().setParagraph().run())}>
                      <Type className="mr-2 h-3.5 w-3.5" />
                      <span>Text</span>
                    </CommandItem>
                    <CommandItem onSelect={() => handleCommand(() => editor.chain().focus().toggleHeading({ level: 1 }).run())}>
                      <Heading1 className="mr-2 h-3.5 w-3.5" />
                      <span>Heading 1</span>
                    </CommandItem>
                    <CommandItem onSelect={() => handleCommand(() => editor.chain().focus().toggleHeading({ level: 2 }).run())}>
                      <Heading2 className="mr-2 h-3.5 w-3.5" />
                      <span>Heading 2</span>
                    </CommandItem>
                    <CommandItem onSelect={() => handleCommand(() => editor.chain().focus().toggleCodeBlock().run())}>
                      <Code className="mr-2 h-3.5 w-3.5" />
                      <span>Code Block</span>
                    </CommandItem>
                  </CommandGroup>
                  <CommandGroup heading="Matter Insights">
                    <CommandItem className="opacity-50">
                      <Search className="mr-2 h-3.5 w-3.5" />
                      <span>Link Fact (soon)</span>
                    </CommandItem>
                    <CommandItem className="opacity-50">
                      <Sparkles className="mr-2 h-3.5 w-3.5" />
                      <span>AI Draft (soon)</span>
                    </CommandItem>
                  </CommandGroup>
                </CommandList>
              </Command>
            </PopoverContent>
          </Popover>
          
          <Separator orientation="vertical" className="mx-1 h-4" />
          
          <ToolbarButton
            onClick={() => editor.chain().focus().undo().run()}
            disabled={readOnly || !editor.can().undo()}
            icon={<Undo className="h-3.5 w-3.5" />}
            label="Undo"
          />
          <ToolbarButton
            onClick={() => editor.chain().focus().redo().run()}
            disabled={readOnly || !editor.can().redo()}
            icon={<Redo className="h-3.5 w-3.5" />}
            label="Redo"
          />
        </div>
      </div>
      <div className="relative p-4">
        <EditorContent editor={editor} />
        <div className="absolute bottom-2 right-2 flex items-center gap-3 text-[9px] uppercase tracking-widest text-muted-foreground tabular-nums">
          <span>{editor.storage.characterCount.words()} words</span>
          <span>{editor.storage.characterCount.characters()} chars</span>
        </div>
      </div>
    </div>
  )
}

function ToolbarButton({ 
  onClick, 
  active = false, 
  disabled = false, 
  icon, 
  label 
}: { 
  onClick: () => void; 
  active?: boolean; 
  disabled?: boolean; 
  icon: ReactNode; 
  label: string 
}) {
  return (
    <Button
      variant="ghost"
      size="sm"
      className={cn(
        "h-7 w-7 p-0",
        active && "bg-primary/10 text-primary hover:bg-primary/20"
      )}
      onClick={(e) => {
        e.preventDefault()
        onClick()
      }}
      disabled={disabled}
      title={label}
    >
      {icon}
    </Button>
  )
}

function textToTiptapDocument(value: string): JSONContent {
  const paragraphs = value.length ? value.split(/\n{2,}/) : [""]
  return {
    type: "doc",
    content: paragraphs.map((paragraph) => {
      const lines = paragraph.split("\n")
      const content = lines.flatMap((line, index) => [
        ...(line ? [{ type: "text", text: line }] : []),
        ...(index < lines.length - 1 ? [{ type: "hardBreak" }] : []),
      ])
      return content.length ? { type: "paragraph", content } : { type: "paragraph" }
    }),
  }
}

function editorPlainText(editor: Editor) {
  return editor.state.doc.textBetween(0, editor.state.doc.content.size, blockSeparator, hardBreakText)
}

function editorSelectionRange(editor: Editor): RichEditorTextRange | null {
  const { from, to } = editor.state.selection
  if (to <= from) return null
  const quote = editor.state.doc.textBetween(from, to, blockSeparator, hardBreakText)
  if (!quote.trim()) return null
  const startOffset = characterLength(editor.state.doc.textBetween(0, from, blockSeparator, hardBreakText))
  return {
    startOffset,
    endOffset: startOffset + characterLength(quote),
    quote,
  }
}

function characterLength(value: string) {
  return Array.from(value).length
}
