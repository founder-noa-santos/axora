using System.Threading.Tasks;

namespace Axora.Logger.Sinks;

/// <summary>
/// Contract for all AXORA Wide Event exporters.
/// Implementations must be thread-safe and non-blocking.
/// </summary>
public interface ISink
{
    Task ExportAsync(WideEventPayload @event);
    Task FlushAsync() => Task.CompletedTask;
}
