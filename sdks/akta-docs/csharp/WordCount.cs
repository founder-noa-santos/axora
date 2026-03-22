using System.Text.RegularExpressions;

namespace OpenAkta.AktaDocs;

public static class WordCount
{
    private static readonly Regex Tokens = new(@"\S+", RegexOptions.Compiled);

    public static int Count(string text)
    {
        var t = text.Trim();
        if (t.Length == 0) return 0;
        return Tokens.Matches(t).Count;
    }
}
