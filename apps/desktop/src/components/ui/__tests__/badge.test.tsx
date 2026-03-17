import { render, screen } from '@testing-library/react'
import { Badge } from '../badge'
import { describe, it, expect } from 'vitest'

describe('Badge', () => {
  it('renders correctly with default variant', () => {
    render(<Badge>Test Badge</Badge>)
    expect(screen.getByText('Test Badge')).toBeInTheDocument()
  })

  it('applies different variants correctly', () => {
    const { rerender } = render(<Badge variant="default">Default</Badge>)
    expect(screen.getByText('Default')).toHaveClass('bg-primary')

    rerender(<Badge variant="secondary">Secondary</Badge>)
    expect(screen.getByText('Secondary')).toHaveClass('bg-secondary')

    rerender(<Badge variant="destructive">Destructive</Badge>)
    expect(screen.getByText('Destructive')).toHaveClass('bg-destructive')

    rerender(<Badge variant="outline">Outline</Badge>)
    expect(screen.getByText('Outline')).toHaveClass('border')
  })

  it('applies correct base styles', () => {
    render(<Badge data-testid="badge">Test</Badge>)
    const badge = screen.getByTestId('badge')
    expect(badge).toHaveClass(
      'inline-flex',
      'items-center',
      'rounded-full',
      'border',
      'px-2.5',
      'py-0.5',
      'text-xs',
      'font-semibold'
    )
  })

  it('accepts custom className', () => {
    render(<Badge className="custom-class">Custom</Badge>)
    expect(screen.getByText('Custom')).toHaveClass('custom-class')
  })

  it('forwards HTML attributes', () => {
    render(<Badge data-testid="test-badge" title="Badge Title">Test</Badge>)
    const badge = screen.getByTestId('test-badge')
    expect(badge).toHaveAttribute('title', 'Badge Title')
  })
})
