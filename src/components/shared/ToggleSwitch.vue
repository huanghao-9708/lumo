<script setup lang="ts">
/**
 * LDL 统一开关：40×22 药丸，开启态 brand-orange。
 * 抽取自设置页手写 toggle，供外观 / 数据同步等分区复用。
 */
const props = withDefaults(defineProps<{
  modelValue: boolean;
  disabled?: boolean;
}>(), {
  disabled: false,
});

const emit = defineEmits<{ 'update:modelValue': [value: boolean] }>();

function toggle() {
  if (!props.disabled) emit('update:modelValue', !props.modelValue);
}
</script>

<template>
  <button
    type="button"
    role="switch"
    :aria-checked="modelValue"
    :disabled="disabled"
    class="w-10 h-[22px] rounded-full transition-colors-smooth relative shrink-0 disabled:opacity-40 cursor-pointer disabled:cursor-default"
    :class="modelValue ? 'bg-brand-orange' : 'bg-border-solid'"
    @click="toggle"
  >
    <div
      class="absolute top-0.5 left-0 w-[18px] h-[18px] rounded-full bg-white shadow transition-transform duration-200 ease-out"
      :class="modelValue ? 'translate-x-[21px]' : 'translate-x-[1px]'"
    ></div>
  </button>
</template>
