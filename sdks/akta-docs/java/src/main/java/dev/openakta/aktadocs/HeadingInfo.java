package dev.openakta.aktadocs;

/** H2/H3 heading in body: 0-based line index of the heading line, depth 2 or 3. */
public record HeadingInfo(int depth, int line0, String text) {}
