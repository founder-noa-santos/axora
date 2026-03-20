using System.Net.Http.Headers;
using System.Text;
using System.Text.Json;
using System.Threading.Tasks;

namespace Axora.Logger.Sinks;

public sealed class HttpSink : ISink
{
    private const int DefaultTimeoutMs = 5000;
    private readonly HttpClient _httpClient;
    private readonly Uri _uri;
    private readonly Dictionary<string, string> _headers;
    private readonly int _timeoutMs;

    public HttpSink(string? url = null, IDictionary<string, string>? headers = null, int timeoutMs = DefaultTimeoutMs, HttpClient? httpClient = null)
    {
        var resolvedUrl = string.IsNullOrWhiteSpace(url) ? Environment.GetEnvironmentVariable("AXORA_SINK_URL") : url;
        if (string.IsNullOrWhiteSpace(resolvedUrl))
        {
            throw new ArgumentException("HttpSink requires a url or AXORA_SINK_URL");
        }

        _uri = new Uri(resolvedUrl);
        _timeoutMs = timeoutMs > 0 ? timeoutMs : DefaultTimeoutMs;
        _httpClient = httpClient ?? new HttpClient();
        _headers = new Dictionary<string, string>
        {
            ["Content-Type"] = "application/json",
        };

        if (headers is not null)
        {
            foreach (var (key, value) in headers)
            {
                _headers[key] = value;
            }
        }

        var token = Environment.GetEnvironmentVariable("AXORA_SINK_TOKEN");
        if (!string.IsNullOrWhiteSpace(token) && !_headers.ContainsKey("Authorization"))
        {
            _headers["Authorization"] = $"Bearer {token}";
        }
    }

    public async Task ExportAsync(WideEventPayload @event)
    {
        using var request = new HttpRequestMessage(HttpMethod.Post, _uri)
        {
            Content = new StringContent(JsonSerializer.Serialize(@event), Encoding.UTF8, "application/json"),
        };

        foreach (var (key, value) in _headers)
        {
            if (key.Equals("Content-Type", StringComparison.OrdinalIgnoreCase))
            {
                request.Content!.Headers.ContentType = MediaTypeHeaderValue.Parse(value);
            }
            else if (key.Equals("Authorization", StringComparison.OrdinalIgnoreCase))
            {
                request.Headers.Authorization = AuthenticationHeaderValue.Parse(value);
            }
            else
            {
                request.Headers.TryAddWithoutValidation(key, value);
            }
        }

        using var cts = new CancellationTokenSource(_timeoutMs);
        using var response = await _httpClient.SendAsync(request, cts.Token).ConfigureAwait(false);
        if (!response.IsSuccessStatusCode)
        {
            throw new HttpRequestException($"HTTP {(int)response.StatusCode} when exporting AXORA Wide Event");
        }
    }
}
