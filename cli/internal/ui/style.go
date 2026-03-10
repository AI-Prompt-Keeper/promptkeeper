package ui

import (
	"charm.land/lipgloss/v2"
)

var (
	// Title for the app name / headers
	Title = lipgloss.NewStyle().
		Bold(true).
		Foreground(lipgloss.Color("13")).
		MarginBottom(1)

	// Subtitle for section headers
	Subtitle = lipgloss.NewStyle().
		Bold(true).
		Foreground(lipgloss.Color("14")).
		MarginTop(1).
		MarginBottom(0)

	// Body for normal text
	Body = lipgloss.NewStyle().
		Foreground(lipgloss.Color("252")).
		MarginLeft(2)

	// Cmd for command names in help
	Cmd = lipgloss.NewStyle().
		Foreground(lipgloss.Color("10")).
		Bold(true)

	// Accent for highlights (e.g. flags)
	Accent = lipgloss.NewStyle().
		Foreground(lipgloss.Color("11"))

	// Box around the whole help
	Box = lipgloss.NewStyle().
		Border(lipgloss.RoundedBorder()).
		BorderForeground(lipgloss.Color("13")).
		Padding(1, 2).
		Margin(1, 0)

	// Error box (red/warm border)
	ErrorBox = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(lipgloss.Color("9")).
			Padding(1, 2).
			Margin(1, 0)

	// Warning box (yellow/amber)
	WarningBox = lipgloss.NewStyle().
			Border(lipgloss.RoundedBorder()).
			BorderForeground(lipgloss.Color("11")).
			Padding(1, 2).
			Margin(1, 0)

	// Success style (green)
	Success = lipgloss.NewStyle().
		Bold(true).
		Foreground(lipgloss.Color("10")).
		MarginTop(0)

	// Error label for "Error:" line
	ErrorLabel = lipgloss.NewStyle().
			Bold(true).
			Foreground(lipgloss.Color("9"))

	// Why / Hint label
	HintLabel = lipgloss.NewStyle().
			Foreground(lipgloss.Color("11"))

	// Example line (dim)
	ExampleLine = lipgloss.NewStyle().
			Foreground(lipgloss.Color("245"))
)
