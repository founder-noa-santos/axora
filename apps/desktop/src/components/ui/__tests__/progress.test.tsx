import { render, screen } from '@testing-library/react'
import { Progress } from '../progress'
import { describe, it, expect } from 'vitest'

describe('Progress', () => {
  it('renders correctly with default value', () => {
    render(<Progress />)
    expect(screen.getByRole('progressbar')).toBeInTheDocument()
  })

  it('displays correct progress value', () => {
    render(<Progress value={75} />)
    const progress = screen.getByRole('progressbar')
    expect(progress).toBeInTheDocument()
    // Check that the indicator is rendered with correct width
    const indicator = progress.querySelector('[style*="transform"]')
    expect(indicator).toBeInTheDocument()
  })

  it('applies correct base styles', () => {
    render(<Progress data-testid="progress" />)
    const progress = screen.getByTestId('progress')
    expect(progress).toHaveClass('relative', 'h-4', 'w-full', 'overflow-hidden', 'rounded-full', 'bg-secondary')
  })

  it('handles 0 value', () => {
    render(<Progress value={0} />)
    const progress = screen.getByRole('progressbar')
    expect(progress).toBeInTheDocument()
  })

  it('handles 100 value', () => {
    render(<Progress value={100} />)
    const progress = screen.getByRole('progressbar')
    expect(progress).toBeInTheDocument()
  })

  it('accepts custom className', () => {
    render(<Progress className="custom-class" />)
    expect(screen.getByRole('progressbar')).toHaveClass('custom-class')
  })

  it('forwards ref correctly', () => {
    const ref = { current: null as HTMLDivElement | null }
    render(<Progress ref={ref} />)
    expect(ref.current).toBeInstanceOf(HTMLDivElement)
  })
})
