package dev.openakta.aktadocs;

public final class WordCount {
    private WordCount() {}

    public static int count(String text) {
        String t = text.strip();
        if (t.isEmpty()) return 0;
        return t.split("\\s+").length;
    }
}
