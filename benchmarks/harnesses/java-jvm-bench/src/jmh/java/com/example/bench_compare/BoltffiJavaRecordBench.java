package com.example.bench_compare;

import com.example.bench_boltffi.BenchBoltFFI;
import com.example.bench_boltffi.BenchmarkUserProfile;
import com.example.bench_boltffi.Line;
import com.example.bench_boltffi.Point;
import com.example.bench_boltffi.ServiceConfig;
import com.example.bench_boltffi.TaggedScores;
import java.util.Arrays;
import java.util.List;
import java.util.Optional;
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
public class BoltffiJavaRecordBench {
    private Point point;
    private Line line;
    private ServiceConfig config;
    private TaggedScores taggedScores;
    private List<BenchmarkUserProfile> profiles;

    @Setup(Level.Trial)
    public void verifyRecordBehavior() {
        point = new Point(3.0, 4.0);
        line = new Line(new Point(0.0, 0.0), point);
        config = new ServiceConfig(
            "benchmark",
            9,
            "eu-west",
            Optional.of("https://edge"),
            Optional.of("https://backup")
        );
        double[] values = new double[256];
        Arrays.fill(values, 1.5);
        taggedScores = new TaggedScores("latency", values);
        profiles = BenchBoltFFI.generateUserProfiles(100);
        require(BenchBoltFFI.echoPoint(point).equals(point), "echo_point");
        require(Math.abs(point.distance() - 5.0) < 0.0001, "point_distance");
        require(point.scale(2.0).equals(new Point(6.0, 8.0)), "point_scale");
        require(BenchBoltFFI.echoLine(line).equals(line), "echo_line");
        require(Math.abs(BenchBoltFFI.lineLength(line) - 5.0) < 0.0001, "line_length");
        require(BenchBoltFFI.echoServiceConfig(config).equals(config), "echo_service_config");
        require(BenchBoltFFI.echoTaggedScores(taggedScores).equals(taggedScores), "echo_tagged_scores");
        require(profiles.size() == 100, "generate_user_profiles_100");
        require(Math.abs(BenchBoltFFI.sumUserScores(profiles) - 7425.0) < 0.0001, "sum_user_scores_100");
    }

    @Benchmark
    public void boltffi_java_echo_point(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoPoint(point));
    }

    @Benchmark
    public void boltffi_java_point_distance(Blackhole blackhole) {
        blackhole.consume(point.distance());
    }

    @Benchmark
    public void boltffi_java_point_scale(Blackhole blackhole) {
        blackhole.consume(point.scale(2.0));
    }

    @Benchmark
    public void boltffi_java_echo_line(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoLine(line));
    }

    @Benchmark
    public void boltffi_java_line_length(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.lineLength(line));
    }

    @Benchmark
    public void boltffi_java_echo_service_config(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoServiceConfig(config));
    }

    @Benchmark
    public void boltffi_java_echo_tagged_scores(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.echoTaggedScores(taggedScores));
    }

    @Benchmark
    public void boltffi_java_generate_user_profiles_100(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.generateUserProfiles(100));
    }

    @Benchmark
    public void boltffi_java_sum_user_scores_100(Blackhole blackhole) {
        blackhole.consume(BenchBoltFFI.sumUserScores(profiles));
    }

    private static void require(boolean condition, String behavior) {
        if (!condition) {
            throw new AssertionError(behavior + " behavior mismatch");
        }
    }
}
