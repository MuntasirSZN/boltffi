import math
import unittest

import demo


class RecordsWithCollectionsTests(unittest.TestCase):
    def test_polygon_functions(self) -> None:
        polygon = demo.Polygon([demo.Point(0.0, 0.0), demo.Point(1.0, 0.0), demo.Point(0.0, 1.0)])

        self.assertEqual(demo.echo_polygon(polygon), polygon)
        self.assertEqual(demo.make_polygon(polygon.points), polygon)
        self.assertEqual(demo.polygon_vertex_count(polygon), 3)

        centroid = demo.polygon_centroid(polygon)

        self.assertTrue(math.isclose(centroid.x, 1.0 / 3.0, rel_tol=0.0, abs_tol=1e-12))
        self.assertTrue(math.isclose(centroid.y, 1.0 / 3.0, rel_tol=0.0, abs_tol=1e-12))

    def test_team_functions(self) -> None:
        team = demo.Team("devs", ["Alice", "Bob"])

        self.assertEqual(demo.echo_team(team), team)
        self.assertEqual(demo.make_team("devs", ["Alice", "Bob"]), team)
        self.assertEqual(demo.team_size(team), 2)

    def test_classroom_functions(self) -> None:
        classroom = demo.Classroom([demo.Person("Mia", 10), demo.Person("Leo", 11)])

        self.assertEqual(demo.echo_classroom(classroom), classroom)
        self.assertEqual(demo.make_classroom(classroom.students), classroom)

    def test_tagged_scores_functions(self) -> None:
        tagged_scores = demo.TaggedScores("math", [90.0, 85.5])
        echoed = demo.echo_tagged_scores(tagged_scores)

        self.assertEqual(echoed.label, "math")
        self.assertEqual(echoed.scores, [90.0, 85.5])
        self.assertTrue(
            math.isclose(
                demo.average_score(demo.TaggedScores("x", [80.0, 100.0])),
                90.0,
                rel_tol=0.0,
                abs_tol=1e-12,
            )
        )
