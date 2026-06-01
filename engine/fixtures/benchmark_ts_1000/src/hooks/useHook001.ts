import { useState } from 'react';
export const useHook001 = () => { const [v, setV] = useState(0); return { v, setV }; };
