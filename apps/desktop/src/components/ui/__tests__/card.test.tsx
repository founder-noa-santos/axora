import { render, screen } from '@testing-library/react'
import { Card, CardHeader, CardTitle, CardDescription, CardContent, CardFooter } from '../card'
import { describe, it, expect } from 'vitest'

describe('Card', () => {
  it('renders card with all parts', () => {
    render(
      <Card>
        <CardHeader>
          <CardTitle>Card Title</CardTitle>
          <CardDescription>Card Description</CardDescription>
        </CardHeader>
        <CardContent>Card Content</CardContent>
        <CardFooter>Card Footer</CardFooter>
      </Card>
    )

    expect(screen.getByText('Card Title')).toBeInTheDocument()
    expect(screen.getByText('Card Description')).toBeInTheDocument()
    expect(screen.getByText('Card Content')).toBeInTheDocument()
    expect(screen.getByText('Card Footer')).toBeInTheDocument()
  })

  it('applies correct base styles', () => {
    render(<Card data-testid="card">Test Card</Card>)
    const card = screen.getByTestId('card')
    expect(card).toHaveClass('rounded-lg', 'border', 'bg-card', 'shadow-sm')
  })

  it('accepts custom className', () => {
    render(<Card className="custom-class">Test Card</Card>)
    expect(screen.getByText('Test Card')).toHaveClass('custom-class')
  })

  it('CardHeader applies correct styles', () => {
    render(<CardHeader>Header</CardHeader>)
    expect(screen.getByText('Header')).toHaveClass('flex', 'flex-col', 'space-y-1.5', 'p-6')
  })

  it('CardTitle renders as h3 with correct styles', () => {
    render(<CardTitle>Title</CardTitle>)
    const title = screen.getByText('Title')
    expect(title.tagName).toBe('H3')
    expect(title).toHaveClass('text-2xl', 'font-semibold', 'tracking-tight')
  })

  it('CardDescription applies muted foreground', () => {
    render(<CardDescription>Description</CardDescription>)
    expect(screen.getByText('Description')).toHaveClass('text-muted-foreground')
  })
})
