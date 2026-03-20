using System.Text.Json;
using System.Threading.Tasks;

namespace Axora.Logger.Sinks;

public sealed class ConsoleSink : ISink
{
    public Task ExportAsync(WideEventPayload @event)
    {
        return Console.Out.WriteLineAsync(JsonSerializer.Serialize(@event));
    }
}
