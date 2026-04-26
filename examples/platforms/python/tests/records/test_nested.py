import math
import unittest

import demo


class NestedRecordsTests(unittest.TestCase):
    def test_line_functions(self) -> None:
        line = demo.Line(demo.Point(0.0, 0.0), demo.Point(3.0, 4.0))

        self.assertEqual(demo.echo_line(line), line)
        self.assertEqual(demo.make_line(0.0, 0.0, 3.0, 4.0), line)
        self.assertTrue(math.isclose(demo.line_length(line), 5.0, rel_tol=0.0, abs_tol=1e-12))

    def test_rect_functions(self) -> None:
        rect = demo.Rect(demo.Point(1.0, 2.0), demo.Dimensions(3.0, 4.0))

        self.assertEqual(demo.echo_rect(rect), rect)
        self.assertTrue(math.isclose(demo.rect_area(rect), 12.0, rel_tol=0.0, abs_tol=1e-12))
