import { zodTextFormat } from 'openai/helpers/zod';

const CycleA = CycleB;
const CycleB = CycleA;

zodTextFormat(CycleA, 'cyclic_schema');
