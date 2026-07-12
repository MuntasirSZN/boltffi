package com.example.bench_compare;

import com.example.bench_boltffi.DataConsumer;
import com.example.bench_boltffi.DataPoint;
import com.example.bench_boltffi.DataProvider;
import java.util.concurrent.TimeUnit;
import java.util.stream.IntStream;
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
public class BoltffiJavaCallbackBench {
    private DataProvider provider100;
    private DataProvider provider1k;

    @Setup(Level.Trial)
    public void setup() {
        provider100 = new FixedDataProvider(100);
        provider1k = new FixedDataProvider(1000);
        try (DataConsumer consumer = new DataConsumer()) {
            consumer.setProvider(provider100);
            if (consumer.computeSum() == 0.0) throw new AssertionError("callback result");
        }
    }

    @Benchmark
    public void boltffi_java_callback_100(Blackhole blackhole) {
        try (DataConsumer consumer = new DataConsumer()) {
            consumer.setProvider(provider100);
            blackhole.consume(consumer.computeSum());
        }
    }

    @Benchmark
    public void boltffi_java_callback_1k(Blackhole blackhole) {
        try (DataConsumer consumer = new DataConsumer()) {
            consumer.setProvider(provider1k);
            blackhole.consume(consumer.computeSum());
        }
    }

    private static final class FixedDataProvider implements DataProvider {
        private final DataPoint[] points;

        private FixedDataProvider(int count) {
            points = IntStream.range(0, count)
                .mapToObj(index -> new DataPoint(
                    (double) index,
                    (double) index * 2.0,
                    (long) index
                ))
                .toArray(DataPoint[]::new);
        }

        @Override
        public int getCount() {
            return points.length;
        }

        @Override
        public DataPoint getItem(int index) {
            return points[index];
        }
    }
}
