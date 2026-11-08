export const faqs = [
  {
    question: 'Does it work with my trainer?',
    answer:
      "ErgLite speaks Bluetooth FTMS, the standard used by most modern trainers. The Zycle ZPro is the project's reference device. The verified compatibility list is updated on GitHub.",
  },
  {
    question: 'Do I need an internet connection?',
    answer:
      'No. Once installed, the app works offline. Your workouts are stored on your computer and nothing is sent to any server.',
  },
  {
    question: 'Is there a subscription or paid version?',
    answer:
      'No. ErgLite is free and open source, under the MIT or Apache-2.0 licence. There are no plans, session limits or in-app purchases.',
  },
  {
    question: 'Where are my workouts stored?',
    answer:
      'In a local SQLite database, in the folder you choose. You can back it up and export your sessions to TCX whenever you want.',
  },
  {
    question: 'Can I import workouts from other apps?',
    answer:
      'Yes. Import workouts in .zwo format and routes in GPX. You can also create your own from scratch with the visual block editor or the route editor.',
  },
  {
    question: 'Is there a mobile version?',
    answer:
      'ErgLite is a desktop app for Windows, macOS and Linux. The session screen is designed to be read from the trainer on a large display, with numbers legible from two metres away.',
  },
  {
    question: 'What do I need to get started?',
    answer:
      'A Bluetooth FTMS-compatible trainer, a computer and the motivation. Optionally, a heart-rate or cadence sensor to complete the metrics.',
  },
] as const;
