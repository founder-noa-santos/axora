def count_words(text: str) -> int:
    t = text.strip()
    if not t:
        return 0
    return len(t.split())
