import unittest

import demo


class RecordsWithEnumsTests(unittest.TestCase):
    def test_task_functions(self) -> None:
        task = demo.Task("ship", demo.Priority.HIGH, False)

        self.assertEqual(demo.echo_task(task), task)
        self.assertEqual(demo.make_task("ship", demo.Priority.CRITICAL), demo.Task("ship", demo.Priority.CRITICAL, False))
        self.assertIs(demo.is_urgent(task), True)
        self.assertIs(demo.is_urgent(demo.Task("later", demo.Priority.LOW, False)), False)

    def test_notification_functions(self) -> None:
        notification = demo.Notification("hello", demo.Priority.LOW, False)

        self.assertEqual(demo.echo_notification(notification), notification)

    def test_repr_c_records_with_c_style_enum_fields(self) -> None:
        header = demo.TaskHeader(7, demo.Priority.HIGH, True)

        self.assertEqual(demo.echo_task_header(header), header)
        self.assertEqual(
            demo.make_critical_task_header(11),
            demo.TaskHeader(11, demo.Priority.CRITICAL, False),
        )

    def test_repr_c_records_with_small_c_style_enum_fields(self) -> None:
        entry = demo.LogEntry(100, demo.LogLevel.WARN, 42)

        self.assertEqual(demo.echo_log_entry(entry), entry)
        self.assertEqual(
            demo.make_error_log_entry(101, 500),
            demo.LogEntry(101, demo.LogLevel.ERROR, 500),
        )
