package com.example.bench_compare;

import com.example.bench_boltffi.BenchBoltFFI;
import com.example.bench_boltffi.Event;
import java.util.concurrent.TimeUnit;
import org.openjdk.jmh.annotations.Benchmark;
import org.openjdk.jmh.annotations.BenchmarkMode;
import org.openjdk.jmh.annotations.Level;
import org.openjdk.jmh.annotations.Mode;
import org.openjdk.jmh.annotations.OutputTimeUnit;
import org.openjdk.jmh.annotations.Scope;
import org.openjdk.jmh.annotations.Setup;
import org.openjdk.jmh.annotations.State;
import org.openjdk.jmh.infra.Blackhole;

@BenchmarkMode(Mode.AverageTime)
@OutputTimeUnit(TimeUnit.NANOSECONDS)
@State(Scope.Thread)
public class BoltffiJavaCustomBench {
    private String email;
    private long timestamp;
    private Event event;

    @Setup(Level.Trial)
    public void verifyCustomTypeBehavior() {
        email = "benchmark@example.com";
        timestamp = 1_700_000_000_123L;
        event = new Event("benchmark", timestamp);
        require(BenchBoltFFI.echoEmail(email).equals(email), "echo_email");
        require(BenchBoltFFI.echoDatetime(timestamp) == timestamp, "echo_datetime");
        require(BenchBoltFFI.echoEvent(event).equals(event), "echo_event");
    }

    @Benchmark
    public void boltffi_java_echo_email(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoEmail(email));
    }

    @Benchmark
    public void boltffi_java_echo_datetime(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoDatetime(timestamp));
    }

    @Benchmark
    public void boltffi_java_echo_event(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoEvent(event));
    }

    private static void require(boolean condition, String behavior) {
        if (!condition) throw new AssertionError(behavior + " behavior mismatch");
    }
}
