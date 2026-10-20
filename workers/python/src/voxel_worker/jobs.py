"""Jobs do worker e o que o WebSocket de progresso ainda tem para enviar."""

import threading
from collections.abc import Callable

type Event = dict[str, object]
type Progress = Callable[[int, int], None]


class Job:
    def __init__(self) -> None:
        self.events: list[Event] = []
        self.done = False
        self.cancelled = threading.Event()


class Book:
    def __init__(self) -> None:
        self._lock = threading.Lock()
        self._jobs: dict[str, Job] = {}

    def open(self, job_id: str) -> Job:
        with self._lock:
            return self.open_locked(job_id)

    def cancel(self, job_id: str) -> None:
        with self._lock:
            self.open_locked(job_id).cancelled.set()

    def push(self, job_id: str, event: Event) -> None:
        with self._lock:
            self.open_locked(job_id).events.append(event)

    def finish(self, job_id: str) -> None:
        with self._lock:
            self.open_locked(job_id).done = True

    def snapshot(self, job_id: str) -> tuple[list[Event], bool, bool]:
        with self._lock:
            job = self._jobs.get(job_id)
            if job is None:
                return [], False, False
            return list(job.events), job.done, True

    def open_locked(self, job_id: str) -> Job:
        job = self._jobs.get(job_id)
        if job is None:
            job = Job()
            self._jobs[job_id] = job
        return job


def reporter(book: Book, job_id: str) -> Progress:
    def report(step: int, total: int) -> None:
        book.push(
            job_id,
            {"event": "progress", "jobId": job_id, "step": step, "total": total},
        )

    return report
