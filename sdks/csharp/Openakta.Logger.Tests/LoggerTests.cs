using System.Collections.Generic;
using System.Threading.Tasks;
using Openakta.Logger.Sinks;
using Xunit;

namespace Openakta.Logger.Tests;

public sealed class LoggerTests
{
    private sealed class CaptureSink : ISink
    {
        public List<WideEventPayload> Events { get; } = [];

        public Task ExportAsync(WideEventPayload @event)
        {
            Events.Add(@event);
            return Task.CompletedTask;
        }
    }

    [Fact]
    public async Task SnapshotsContextAndFinalizesEvent()
    {
        var sink = new CaptureSink();
        var logger =
            new LoggerBuilder()
                .Service("my-api")
                .Environment("staging")
                .AddSink(sink)
                .DefaultContext(new Dictionary<string, object?> { ["region"] = "eu-west-1" })
                .Build();

        var details = new Dictionary<string, object?> { ["step"] = 1 };
        var wideEvent = logger.StartEvent("user.login");
        wideEvent.AppendContext(new Dictionary<string, object?> { ["details"] = details });
        await wideEvent.EmitAsync();
        details["step"] = 2;

        Assert.Single(sink.Events);
        Assert.Equal("my-api", sink.Events[0].Service);
        Assert.Equal("staging", sink.Events[0].Environment);
        Assert.Equal("ok", sink.Events[0].Status);
        var capturedDetails = Assert.IsType<Dictionary<string, object?>>(sink.Events[0].Context["details"]);
        Assert.Equal(1L, capturedDetails["step"]);

        Assert.Throws<InvalidOperationException>(() => wideEvent.AppendContext(new Dictionary<string, object?> { ["later"] = true }));
    }

    [Fact]
    public async Task TraceAsyncCapturesErrors()
    {
        var sink = new CaptureSink();
        var logger = new LoggerBuilder().Service("svc").AddSink(sink).Build();

        await Assert.ThrowsAsync<InvalidOperationException>(
            async () =>
                await logger.TraceAsync<int>(
                    "job.fail",
                    _ => Task.FromException<int>(new InvalidOperationException("boom"))));

        Assert.Equal("error", sink.Events[0].Level);
        Assert.Equal("error", sink.Events[0].Status);
    }
}
