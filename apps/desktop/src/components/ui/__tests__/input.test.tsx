import { render, screen } from '@testing-library/react'
import { Input } from '../input'
import { describe, it, expect } from 'vitest'

describe('Input', () => {
  it('renders correctly', () => {
    render(<Input placeholder="Enter text" />)
    expect(screen.getByPlaceholderText('Enter text')).toBeInTheDocument()
  })

  it('applies correct base styles', () => {
    render(<Input data-testid="input" />)
    const input = screen.getByTestId('input')
    expect(input).toHaveClass(
      'flex',
      'h-10',
      'w-full',
      'rounded-md',
      'border',
      'border-input',
      'bg-background',
      'px-3',
      'py-2',
      'text-sm'
    )
  })

  it('supports different input types', () => {
    const { rerender } = render(<Input type="text" />)
    expect(screen.getByRole('textbox')).toHaveAttribute('type', 'text')

    rerender(<Input type="email" />)
    expect(screen.getByRole('textbox')).toHaveAttribute('type', 'email')

    rerender(<Input type="password" />)
    expect(screen.getByDisplayValue('')).toHaveAttribute('type', 'password')
  })

  it('forwards ref correctly', () => {
    const ref = { current: null as HTMLInputElement | null }
    render(<Input ref={ref} />)
    expect(ref.current).toBeInstanceOf(HTMLInputElement)
  })

  it('is disabled when disabled prop is passed', () => {
    render(<Input disabled />)
    expect(screen.getByRole('textbox')).toBeDisabled()
  })

  it('accepts custom className', () => {
    render(<Input className="custom-class" />)
    expect(screen.getByRole('textbox')).toHaveClass('custom-class')
  })
})
