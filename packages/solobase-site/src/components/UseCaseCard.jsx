export default function UseCaseCard({ title, description, tagLabel, color }) {
  return (
    <div
      class="bg-white border border-gray-200 rounded-xl p-6 transition-all duration-200 hover:-translate-y-0.5 hover:shadow-md hover:border-gray-300"
    >
      <span
        class="inline-block text-xs font-semibold uppercase tracking-wide px-2.5 py-0.5 rounded-full mb-3"
        style={{ background: color.tagBg, color: color.tagText }}
      >
        {tagLabel}
      </span>
      <h3 class="text-lg font-semibold text-gray-800 mb-2">{title}</h3>
      <p class="text-gray-500 text-sm leading-relaxed">{description}</p>
    </div>
  );
}
