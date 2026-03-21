using System.Threading.Tasks;

namespace Openakta.Logger.Sinks;

/// <summary>
/// Contract for all OPENAKTA Wide Event exporters.
/// Implementations must be thread-safe and non-blocking.
/// </summary>
public interface ISink
{
    Task ExportAsync(WideEventPayload @event);
    Task FlushAsync() => Task.CompletedTask;
}
